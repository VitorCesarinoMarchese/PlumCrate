use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    time::{Duration, SystemTime},
};

const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_CACHE_BYTES: u64 = 128 * 1024 * 1024;
const FRESH_FOR: Duration = Duration::from_secs(7 * 24 * 60 * 60);

fn key(url: &str) -> String {
    format!("ts4-photo-{:x}", Sha256::digest(url.as_bytes()))
}

fn decode_image(bytes: &[u8]) -> Option<image::DynamicImage> {
    use image::ImageDecoder as _;
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(64 * 1024 * 1024);
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    reader.limits(limits.clone());
    let mut decoder = reader.into_decoder().ok()?;
    limits.reserve(decoder.total_bytes()).ok()?;
    decoder.set_limits(limits).ok()?;
    let orientation = decoder.orientation().ok()?;
    let mut image = image::DynamicImage::from_decoder(decoder).ok()?;
    image.apply_orientation(orientation);
    Some(image)
}

fn read_cached(path: &Path) -> Option<(fs::Metadata, image::DynamicImage)> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_IMAGE_BYTES {
        return None;
    }
    let image = decode_image(&fs::read(path).ok()?)?;
    Some((metadata, image))
}

pub fn fetch(directory: &Path, url: &str) -> Result<image::DynamicImage, String> {
    let path = directory.join(key(url));
    let cached = read_cached(&path);
    if cached
        .as_ref()
        .and_then(|(metadata, _)| metadata.modified().ok())
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < FRESH_FOR)
    {
        return Ok(cached.expect("fresh cached image").1);
    }
    let download = || -> Result<(Vec<u8>, image::DynamicImage), String> {
        let agent = ureq::builder()
            .timeout(Duration::from_secs(8))
            .timeout_connect(Duration::from_secs(5))
            .build();
        let response = agent
            .get(url)
            .set("User-Agent", "PlumCrate/0.1 photo cache")
            .call()
            .map_err(|_| "Preview request failed")?;
        let mut bytes = vec![];
        response
            .into_reader()
            .take(MAX_IMAGE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Preview response failed")?;
        if bytes.len() as u64 > MAX_IMAGE_BYTES {
            return Err("Unsupported or oversized preview".into());
        }
        let image = decode_image(&bytes).ok_or("Unsupported or oversized preview")?;
        Ok((bytes, image))
    };
    let (bytes, image) = match download() {
        Ok(result) => result,
        Err(error) => return cached.map(|(_, image)| image).ok_or(error),
    };
    fs::create_dir_all(directory).map_err(|_| "Cannot create photo cache")?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(directory).map_err(|_| "Cannot write photo cache")?;
    temporary
        .write_all(&bytes)
        .map_err(|_| "Cannot write photo cache")?;
    temporary
        .persist(&path)
        .map_err(|_| "Cannot save photo cache")?;
    prune(directory, &path);
    Ok(image)
}

fn prune(directory: &Path, keep: &Path) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    let mut files = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_str()?;
            let hash = name.strip_prefix("ts4-photo-")?;
            if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return None;
            }
            let metadata = fs::symlink_metadata(&path).ok()?;
            if !metadata.is_file() {
                return None;
            }
            Some((
                path,
                metadata.len(),
                metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            ))
        })
        .collect::<Vec<_>>();
    let mut total: u64 = files.iter().map(|(_, size, _)| size).sum();
    files.sort_by_key(|(_, _, modified)| *modified);
    for (path, size, _) in files {
        if total <= MAX_CACHE_BYTES {
            break;
        }
        if path != keep && fs::remove_file(path).is_ok() {
            total -= size;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_photo_is_available_offline_and_invalid_cache_is_not_used() {
        let dir = tempfile::tempdir().unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/photo", listener.local_addr().unwrap());
        drop(listener);
        let path = dir.path().join(key(&url));
        image::RgbaImage::from_pixel(20, 30, image::Rgba([10, 100, 80, 255]))
            .save_with_format(&path, image::ImageFormat::Png)
            .unwrap();
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(SystemTime::UNIX_EPOCH))
            .unwrap();
        assert_eq!(
            fetch(dir.path(), &url)
                .unwrap()
                .to_rgba8()
                .get_pixel(0, 0)
                .0,
            [10, 100, 80, 255]
        );
        fs::write(&path, b"broken cache").unwrap();
        assert!(fetch(dir.path(), &url).is_err());
    }
    #[test]
    fn cache_keys_distinguish_full_urls_and_reject_broken_images() {
        assert_ne!(
            key("https://example/a?size=1"),
            key("https://example/a?size=2")
        );
        assert!(decode_image(b"not an image").is_none());
    }
    #[test]
    fn pruning_retains_unrelated_files_and_current_photo() {
        let dir = tempfile::tempdir().unwrap();
        let unrelated = dir.path().join("user.png");
        fs::write(&unrelated, b"original").unwrap();
        let old = dir.path().join(key("old"));
        fs::File::create(&old)
            .unwrap()
            .set_len(MAX_CACHE_BYTES)
            .unwrap();
        let keep = dir.path().join(key("current"));
        fs::write(&keep, b"current").unwrap();
        prune(dir.path(), &keep);
        assert!(!old.exists());
        assert!(keep.exists());
        assert_eq!(fs::read(unrelated).unwrap(), b"original");
    }
}
