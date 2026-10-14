use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::mpsc,
};

#[derive(Debug, PartialEq)]
pub enum State {
    Pending,
    Ready(egui::load::SizedTexture),
    Missing,
}

#[derive(Default)]
pub struct Previews {
    recent: VecDeque<String>,
    generation: Option<u64>,
    cache: Option<PathBuf>,
    remote: HashMap<String, Result<String, mpsc::Receiver<Result<PathBuf, String>>>>,
}

impl Previews {
    pub fn with_cache(directory: PathBuf) -> Self {
        Self {
            cache: Some(directory),
            ..Self::default()
        }
    }

    fn cached_uri(&mut self, ctx: &egui::Context, uri: &str) -> Result<String, State> {
        let Some(directory) = self.cache.clone() else {
            return Ok(uri.into());
        };
        if !uri.starts_with("http://") && !uri.starts_with("https://") {
            return Ok(uri.into());
        }
        for state in self.remote.values_mut() {
            let Err(receiver) = state else { continue };
            match receiver.try_recv() {
                Ok(result) => {
                    *state = Ok(result
                        .ok()
                        .and_then(|path| resolve_uri(path.to_str(), None))
                        .unwrap_or_default());
                }
                Err(mpsc::TryRecvError::Disconnected) => *state = Ok(String::new()),
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if let Some(state) = self.remote.get(uri) {
            return match state {
                Ok(path) if path.is_empty() => Err(State::Missing),
                Ok(path) => Ok(path.clone()),
                Err(_) => Err(State::Pending),
            };
        }
        if self.remote.values().filter(|state| state.is_err()).count() >= 4 {
            return Err(State::Pending);
        }
        let (sender, receiver) = mpsc::channel();
        let url = uri.to_owned();
        let wake = ctx.clone();
        if std::thread::Builder::new()
            .name("photo-cache".into())
            .spawn(move || {
                let result = crate::photo_cache::fetch(&directory, &url);
                let _ = sender.send(result);
                wake.request_repaint();
            })
            .is_err()
        {
            return Err(State::Missing);
        }
        self.remote.insert(uri.into(), Err(receiver));
        Err(State::Pending)
    }
    pub fn begin_frame(&mut self, ctx: &egui::Context, generation: u64) {
        if self.generation != Some(generation) {
            for uri in self.recent.drain(..) {
                ctx.forget_image(&uri);
            }
            self.remote.clear();
            self.generation = Some(generation);
        }
    }

    pub fn load(
        &mut self,
        ctx: &egui::Context,
        preview: Option<&str>,
        root: Option<&Path>,
    ) -> State {
        let Some(uri) = resolve_uri(preview, root) else {
            return State::Missing;
        };
        let uri = match self.cached_uri(ctx, &uri) {
            Ok(uri) => uri,
            Err(state) => return state,
        };
        if let Some(index) = self.recent.iter().position(|cached| *cached == uri) {
            self.recent.remove(index);
        }
        self.recent.push_back(uri.clone());
        while self.recent.len() > 32 {
            ctx.forget_image(&self.recent.pop_front().expect("cached preview"));
        }
        match ctx.try_load_texture(
            &uri,
            egui::TextureOptions::LINEAR,
            egui::load::SizeHint::Size {
                width: 1024,
                height: 1024,
                maintain_aspect_ratio: true,
            },
        ) {
            Ok(egui::load::TexturePoll::Ready { texture }) => State::Ready(texture),
            Ok(egui::load::TexturePoll::Pending { .. }) => State::Pending,
            Err(_) => State::Missing,
        }
    }
}

pub fn resolve_uri(preview: Option<&str>, root: Option<&Path>) -> Option<String> {
    let value = preview?.trim();
    if value.is_empty() {
        return None;
    }
    let path = if let Ok(url) = url::Url::parse(value) {
        match url.scheme() {
            "http" | "https"
                if url.host_str().is_some()
                    && url.username().is_empty()
                    && url.password().is_none() =>
            {
                return Some(url.into());
            }
            "file" => url.to_file_path().ok()?,
            _ => return None,
        }
    } else {
        if value.contains("://") {
            return None;
        }
        let path = Path::new(value);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            root?.join("Mods").join(path)
        }
    };
    let path = path.to_string_lossy();
    Some(if cfg!(windows) {
        format!("file:///{}", path.replace('\\', "/"))
    } else {
        format!("file://{path}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait_for_preview(ctx: &egui::Context, previews: &mut Previews, uri: &str) -> State {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let state = previews.load(ctx, Some(uri), None);
            if state != State::Pending {
                return state;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "preview did not finish"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    #[test]
    fn completed_offscreen_downloads_release_slots() {
        let temp = tempfile::tempdir().unwrap();
        let photo = temp.path().join("photo.png");
        image::RgbaImage::from_pixel(20, 30, image::Rgba([10, 100, 80, 255]))
            .save(&photo)
            .unwrap();
        let ctx = egui::Context::default();
        crate::ui::setup(&ctx);
        let mut previews = Previews::with_cache(temp.path().join("cache"));
        previews.begin_frame(&ctx, 1);
        let mut senders = vec![];
        for index in 0..4 {
            let (sender, receiver) = mpsc::channel();
            senders.push(sender);
            previews.remote.insert(
                format!("https://fixture.invalid/{index}.png"),
                Err(receiver),
            );
        }
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let next = format!("http://{}/next.png", server.local_addr().unwrap());
        drop(server);
        assert_eq!(previews.load(&ctx, Some(&next), None), State::Pending);
        assert!(
            !previews.remote.contains_key(&next),
            "active downloads stay bounded"
        );
        senders[0].send(Ok(photo.clone())).unwrap();
        senders[1].send(Err("broken image".into())).unwrap();
        drop(senders);

        // Scroll to a new card without drawing any of the previous four.
        assert_eq!(previews.load(&ctx, Some(&next), None), State::Pending);
        assert!(
            previews.remote.contains_key(&next),
            "finished offscreen requests must free slots"
        );
        assert!(matches!(
            wait_for_preview(&ctx, &mut previews, "https://fixture.invalid/0.png"),
            State::Ready(_)
        ));
        for index in 1..4 {
            assert_eq!(
                previews.load(
                    &ctx,
                    Some(&format!("https://fixture.invalid/{index}.png")),
                    None
                ),
                State::Missing
            );
        }
        assert_eq!(wait_for_preview(&ctx, &mut previews, &next), State::Missing);
    }

    #[test]
    fn remote_photo_survives_rescan_and_app_restart_offline() {
        use std::io::{Read, Write};
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("source.png");
        image::RgbaImage::from_pixel(20, 30, image::Rgba([10, 100, 80, 255]))
            .save(&path)
            .unwrap();
        let png = std::fs::read(&path).unwrap();
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/photo.png", server.local_addr().unwrap());
        let requests = std::thread::spawn(move || {
            let (mut socket, _) = server.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut request = [0; 8192];
            let count = socket.read(&mut request).unwrap();
            assert!(
                !String::from_utf8_lossy(&request[..count])
                    .to_lowercase()
                    .contains("referer:")
            );
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", png.len()).unwrap();
            socket.write_all(&png).unwrap();
        });
        let cache = temp.path().join("cache");
        let ctx = egui::Context::default();
        crate::ui::setup(&ctx);
        let mut previews = Previews::with_cache(cache.clone());
        assert!(matches!(
            wait_for_preview(&ctx, &mut previews, &url),
            State::Ready(_)
        ));
        requests.join().unwrap();
        previews.begin_frame(&ctx, 2);
        assert!(matches!(
            wait_for_preview(&ctx, &mut previews, &url),
            State::Ready(_)
        ));
        drop(previews);
        let ctx = egui::Context::default();
        crate::ui::setup(&ctx);
        let mut reopened = Previews::with_cache(cache);
        assert!(matches!(
            wait_for_preview(&ctx, &mut reopened, &url),
            State::Ready(_)
        ));
    }

    #[test]
    fn loads_local_photo_and_falls_back_for_corrupt_or_missing_images() {
        let temp = tempfile::tempdir().unwrap();
        let photo = temp.path().join("Café preview #1.png");
        image::RgbaImage::from_pixel(80, 40, image::Rgba([10, 100, 80, 255]))
            .save(&photo)
            .unwrap();
        let ctx = egui::Context::default();
        crate::ui::setup(&ctx);
        let mut previews = Previews::default();
        match wait_for_preview(&ctx, &mut previews, photo.to_str().unwrap()) {
            State::Ready(texture) => assert_eq!(texture.size, egui::vec2(80.0, 40.0)),
            state => panic!("photo was not rendered: {state:?}"),
        }
        let corrupt = temp.path().join("broken.png");
        std::fs::write(&corrupt, b"not an image").unwrap();
        assert_eq!(
            wait_for_preview(&ctx, &mut previews, corrupt.to_str().unwrap()),
            State::Missing
        );
        assert_eq!(
            wait_for_preview(
                &ctx,
                &mut previews,
                temp.path().join("missing.png").to_str().unwrap()
            ),
            State::Missing
        );
        assert_eq!(previews.load(&ctx, None, None), State::Missing);
    }

    #[test]
    fn supports_jpeg_webp_and_refreshes_changed_local_previews() {
        let temp = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        crate::ui::setup(&ctx);
        let mut previews = Previews::default();
        previews.begin_frame(&ctx, 1);
        for extension in ["jpg", "webp"] {
            let path = temp.path().join(format!("preview.{extension}"));
            image::RgbImage::from_pixel(60, 40, image::Rgb([20, 100, 80]))
                .save(&path)
                .unwrap();
            assert!(matches!(
                wait_for_preview(&ctx, &mut previews, path.to_str().unwrap()),
                State::Ready(_)
            ));
        }
        let path = temp.path().join("preview.jpg");
        image::RgbImage::from_pixel(20, 30, image::Rgb([20, 100, 80]))
            .save(&path)
            .unwrap();
        previews.begin_frame(&ctx, 2);
        match wait_for_preview(&ctx, &mut previews, path.to_str().unwrap()) {
            State::Ready(texture) => assert_eq!(texture.size, egui::vec2(20.0, 30.0)),
            state => panic!("refresh failed: {state:?}"),
        }
    }

    #[test]
    fn remote_images_and_redirects_send_no_referrer() {
        use std::io::{Read, Write};
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("preview.png");
        image::RgbaImage::from_pixel(20, 30, image::Rgba([10, 100, 80, 255]))
            .save(&path)
            .unwrap();
        let png = std::fs::read(path).unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/redirect", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let mut requests = vec![];
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            for index in 0..2 {
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(std::time::Instant::now() < deadline, "no HTTP request");
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
                        Err(error) => panic!("HTTP accept: {error}"),
                    }
                };
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = vec![];
                let mut byte = [0];
                while !bytes.ends_with(b"\r\n\r\n") {
                    stream.read_exact(&mut byte).unwrap();
                    bytes.push(byte[0]);
                    assert!(bytes.len() < 8192);
                }
                requests.push(String::from_utf8(bytes).unwrap());
                if index == 0 {
                    stream.write_all(b"HTTP/1.1 302 Found\r\nLocation: /cover.png\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                } else {
                    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", png.len()).unwrap();
                    stream.write_all(&png).unwrap();
                }
            }
            requests
        });
        let ctx = egui::Context::default();
        crate::ui::setup(&ctx);
        let mut previews = Previews::with_cache(temp.path().join("cache"));
        assert!(matches!(
            wait_for_preview(&ctx, &mut previews, &url),
            State::Ready(_)
        ));
        let requests = server.join().unwrap();
        assert!(requests[0].starts_with("GET /redirect "));
        assert!(requests[1].starts_with("GET /cover.png "));
        for request in requests {
            assert!(
                !request.to_lowercase().contains("\r\nreferer:"),
                "referrer was sent"
            );
        }
    }

    #[test]
    fn resolves_local_paths_and_saved_http_urls() {
        let root = Path::new("/fixture/The Sims 4");
        assert_eq!(
            resolve_uri(Some(" /fixture/Café preview #1.png "), None).as_deref(),
            Some("file:///fixture/Café preview #1.png")
        );
        assert_eq!(
            resolve_uri(Some("Pack/preview.png"), Some(root)).as_deref(),
            Some("file:///fixture/The Sims 4/Mods/Pack/preview.png")
        );
        assert_eq!(
            resolve_uri(Some("file:///fixture/Caf%C3%A9%20preview.png"), None).as_deref(),
            Some("file:///fixture/Café preview.png")
        );
        assert_eq!(
            resolve_uri(Some("HTTPS://media.example/cover.jpg?width=600"), None).as_deref(),
            Some("https://media.example/cover.jpg?width=600")
        );
    }

    #[test]
    fn rejects_missing_previews_and_unsupported_or_invalid_urls() {
        for value in [
            None,
            Some(" "),
            Some("javascript:alert(1)"),
            Some("blob:browser-only"),
            Some("asset://old-webview"),
            Some("https://"),
            Some("https://user:secret@media.example/cover.jpg"),
            Some("relative.png"),
        ] {
            assert!(
                resolve_uri(value, None).is_none(),
                "invalid preview {value:?}"
            );
        }
    }
}
