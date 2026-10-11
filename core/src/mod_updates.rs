//! User-confirmed installed releases and read-only stable project release checks.
use crate::{
    curseforge_client::{
        classify_status, CurseForgeClient, CurseForgeTransport, SourceLookupError, SIMS4_GAME_ID,
    },
    error::{ErrorCode, ManagerError},
    fs_scope,
    managed_storage::{self, ModMetadata, SourceAttachmentMetadata},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashSet, path::Path};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledRelease {
    pub project_id: u64,
    pub file_id: u64,
    pub content_digest: String,
    pub confirmed_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub project_id: u64,
    pub file_id: u64,
    pub name: String,
    pub file_name: String,
    pub published_at: DateTime<Utc>,
    pub game_versions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateStatus {
    Untracked,
    LocalContentChanged,
    InstalledReleaseUnavailable,
    Incomplete,
    NoNewerRelease,
    Available(Release),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateReport {
    pub project_id: u64,
    pub source_url: String,
    pub content_digest: String,
    pub installed: Option<Release>,
    pub releases: Vec<Release>,
    pub complete: bool,
    pub status: UpdateStatus,
}

fn invalid(message: &str) -> ManagerError {
    ManagerError::new(ErrorCode::SourceInvalidResponse, message)
}

fn lookup_error(error: SourceLookupError) -> ManagerError {
    let code = match error {
        SourceLookupError::MissingApiKey => ErrorCode::SourceMissingApiKey,
        SourceLookupError::Unauthorized => ErrorCode::SourceUnauthorized,
        SourceLookupError::RateLimited => ErrorCode::SourceRateLimited,
        SourceLookupError::Network => ErrorCode::SourceNetwork,
        SourceLookupError::InvalidResponse => ErrorCode::SourceInvalidResponse,
        SourceLookupError::ProviderUnavailable => ErrorCode::SourceProviderUnavailable,
        SourceLookupError::NoUsableEvidence => ErrorCode::SourceNoUsableEvidence,
    };
    ManagerError::new(code, "Provider update check failed")
}

fn attachment(meta: &ModMetadata) -> Result<&SourceAttachmentMetadata, ManagerError> {
    meta.source_attachment
        .as_ref()
        .filter(|source| {
            source.provider_id == "curseforge"
                && source.project_id.is_some_and(|id| id > 0)
                && meta.source_url.as_deref() == Some(source.source_url.as_str())
        })
        .ok_or_else(|| {
            ManagerError::new(
                ErrorCode::SourceNoUsableEvidence,
                "Attach a CurseForge project before checking updates",
            )
        })
}

fn content_digest(managed_root: &Path, mod_id: &str) -> Result<String, ManagerError> {
    fs_scope::component(mod_id)?;
    #[cfg(unix)]
    {
        fs_scope::Directory::open(&managed_root.join("mods").join(mod_id).join("files"))?
            .mod_content_digest()
    }
    #[cfg(not(unix))]
    {
        Err(ManagerError::new(
            ErrorCode::InternalError,
            "Release checks require Linux",
        ))
    }
}

fn parse_json(status: u16, body: &str) -> Result<Value, ManagerError> {
    classify_status(status).map_err(lookup_error)?;
    serde_json::from_str(body).map_err(|_| invalid("Invalid provider response"))
}

/// A project page must come from the provider, use HTTPS, and belong to CurseForge.
fn project_page(value: &Value, project_id: u64) -> Result<String, ManagerError> {
    let data = &value["data"];
    if data["id"].as_u64() != Some(project_id)
        || data["gameId"].as_u64() != Some(u64::from(SIMS4_GAME_ID))
    {
        return Err(invalid(
            "Provider project identity does not match this Sims 4 source",
        ));
    }
    let url = data["links"]["websiteUrl"].as_str().unwrap_or_default();
    let valid = [
        "https://www.curseforge.com/sims4/",
        "https://curseforge.com/sims4/",
    ]
    .iter()
    .any(|prefix| url.starts_with(prefix) && url.len() > prefix.len());
    if !valid
        || url
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(invalid(
            "Provider did not supply a valid CurseForge project page",
        ));
    }
    Ok(url.to_string())
}

// None means an explicitly excluded channel/availability/status; Err means uncertain data.
fn parse_release(value: &Value, project_id: u64) -> Result<Option<Release>, ()> {
    let id = value["id"].as_u64().filter(|id| *id > 0).ok_or(())?;
    if value["modId"].as_u64() != Some(project_id) {
        return Err(());
    }
    let channel = value["releaseType"].as_u64().ok_or(())?;
    if channel == 2 || channel == 3 {
        return Ok(None);
    }
    if channel != 1 {
        return Err(());
    }
    if !value["isAvailable"].as_bool().ok_or(())? {
        return Ok(None);
    }
    let early_access = value
        .get("isEarlyAccessContent")
        .map(|value| value.as_bool().ok_or(()))
        .transpose()?
        .unwrap_or(false);
    if early_access {
        return Ok(None);
    }
    let status = value["fileStatus"].as_u64().ok_or(())?;
    if status != 4 && status != 10 {
        // Unknown future statuses cannot establish a complete listing.
        return if (1..=16).contains(&status) {
            Ok(None)
        } else {
            Err(())
        };
    }
    let date = DateTime::parse_from_rfc3339(value["fileDate"].as_str().ok_or(())?)
        .map_err(|_| ())?
        .with_timezone(&Utc);
    let file_name = value["fileName"]
        .as_str()
        .filter(|name| !name.trim().is_empty())
        .ok_or(())?
        .to_string();
    let name = value["displayName"]
        .as_str()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(&file_name)
        .to_string();
    let game_versions = value["gameVersions"]
        .as_array()
        .ok_or(())?
        .iter()
        .map(|value| value.as_str().map(str::to_string).ok_or(()))
        .collect::<Result<_, _>>()?;
    Ok(Some(Release {
        project_id,
        file_id: id,
        name,
        file_name,
        published_at: date,
        game_versions,
    }))
}

fn release_listing<T: CurseForgeTransport>(
    client: &CurseForgeClient<T>,
    project_id: u64,
) -> Result<(Vec<Release>, bool), ManagerError> {
    let mut releases = Vec::new();
    let mut seen = HashSet::new();
    let mut total = None;
    let mut complete = true;
    let mut index = 0_u64;
    for page_number in 0..10 {
        let response = client.get_update_response(&format!(
            "mods/{project_id}/files?pageSize=50&index={index}"
        ));
        let value = match response
            .map_err(lookup_error)
            .and_then(|(status, body)| parse_json(status, &body))
        {
            Ok(value) => value,
            Err(error)
                if page_number > 0
                    && matches!(
                        error.code,
                        ErrorCode::SourceNetwork | ErrorCode::SourceProviderUnavailable
                    ) =>
            {
                return Ok((releases, false));
            }
            Err(error) => return Err(error),
        };
        let data = value["data"]
            .as_array()
            .ok_or_else(|| invalid("Missing provider files"))?;
        for record in data {
            let Some(id) = record["id"].as_u64() else {
                complete = false;
                continue;
            };
            if !seen.insert(id) {
                releases.retain(|release: &Release| release.file_id != id);
                complete = false;
                continue;
            }
            match parse_release(record, project_id) {
                Ok(Some(release)) => releases.push(release),
                Ok(None) => (),
                Err(()) => complete = false,
            }
        }
        let pagination = &value["pagination"];
        let count = data.len() as u64;
        let reported_total = pagination["totalCount"].as_u64();
        if pagination["index"].as_u64() != Some(index)
            || pagination["pageSize"].as_u64() != Some(50)
            || pagination["resultCount"].as_u64() != Some(count)
            || count > 50
            || reported_total.is_none()
            || total.is_some_and(|total| Some(total) != reported_total)
            || reported_total.is_some_and(|total| total < index + count)
        {
            return Ok((releases, false));
        }
        total = reported_total;
        index += count;
        if Some(index) == total {
            return Ok((releases, complete));
        }
        if count != 50 {
            return Ok((releases, false));
        }
    }
    Ok((releases, false))
}

/// Checks never write metadata, download files, or infer installed identity from source suggestions.
pub fn check_mod_updates<T: CurseForgeTransport>(
    managed_root: &Path,
    mod_id: &str,
    client: &CurseForgeClient<T>,
) -> Result<UpdateReport, ManagerError> {
    let meta = managed_storage::read_managed_mod(managed_root, mod_id)?;
    let source = attachment(&meta)?;
    let project_id = source.project_id.unwrap();
    let digest = content_digest(managed_root, mod_id)?;
    let (status, body) = client
        .get_update_response(&format!("mods/{project_id}"))
        .map_err(lookup_error)?;
    let page = project_page(&parse_json(status, &body)?, project_id)?;
    // The saved attached page remains the exact confirmation binding. Redirected/replaced
    // provider project pages require attaching the current source explicitly.
    if page != source.source_url {
        return Err(invalid(
            "Provider project page differs from the attached source",
        ));
    }
    let mut report = UpdateReport {
        project_id,
        source_url: page,
        content_digest: digest,
        installed: None,
        releases: Vec::new(),
        complete: false,
        status: UpdateStatus::Untracked,
    };
    if let Some(installed) = &source.installed_release {
        if installed.project_id != project_id {
            report.status = UpdateStatus::InstalledReleaseUnavailable;
        } else if installed.content_digest != report.content_digest {
            report.status = UpdateStatus::LocalContentChanged;
        } else {
            let (status, body) = client
                .get_update_response(&format!("mods/{project_id}/files/{}", installed.file_id))
                .map_err(lookup_error)?;
            report.installed = if status == 404 {
                None
            } else {
                classify_status(status).map_err(lookup_error)?;
                serde_json::from_str::<Value>(&body)
                    .ok()
                    .and_then(|value| parse_release(&value["data"], project_id).ok().flatten())
                    .filter(|release| release.file_id == installed.file_id)
            };
            if report.installed.is_none() {
                report.status = UpdateStatus::InstalledReleaseUnavailable;
            }
        }
    }
    let (mut releases, complete) = release_listing(client, project_id)?;
    releases.sort_by(|left, right| {
        right
            .published_at
            .cmp(&left.published_at)
            .then(left.file_id.cmp(&right.file_id))
    });
    report.releases = releases;
    report.complete = complete;
    if let Some(installed) = &report.installed {
        if report
            .releases
            .iter()
            .any(|release| release.file_id == installed.file_id && release != installed)
        {
            report.complete = false;
        }
        report.status = match report.releases.iter().find(|release| {
            release.file_id != installed.file_id && release.published_at > installed.published_at
        }) {
            Some(release) => UpdateStatus::Available(release.clone()),
            None if report.complete => UpdateStatus::NoNewerRelease,
            None => UpdateStatus::Incomplete,
        };
    }
    let final_digest = content_digest(managed_root, mod_id)?;
    if final_digest != report.content_digest {
        report.installed = None;
        report.status = if source.installed_release.is_some() {
            UpdateStatus::LocalContentChanged
        } else {
            UpdateStatus::Untracked
        };
        report.content_digest = final_digest;
    }
    Ok(report)
}

/// Invoke only after the user confirms installed identity and project-wide stable release notices.
pub fn confirm_installed_release(
    managed_root: &Path,
    mod_id: &str,
    report: &UpdateReport,
    release: &Release,
) -> Result<ModMetadata, ManagerError> {
    let _guard = crate::operation::acquire(managed_root)?;
    let mut meta = managed_storage::read_managed_mod(managed_root, mod_id)?;
    let source = attachment(&meta)?;
    if source.project_id != Some(report.project_id)
        || source.source_url != report.source_url
        || release.project_id != report.project_id
        || !report.releases.contains(release)
        || content_digest(managed_root, mod_id)? != report.content_digest
    {
        return Err(ManagerError::new(
            ErrorCode::InvalidPath,
            "Source, release choice, or managed content changed; check again",
        ));
    }
    let installed = InstalledRelease {
        project_id: release.project_id,
        file_id: release.file_id,
        content_digest: report.content_digest.clone(),
        confirmed_at: Utc::now().to_rfc3339(),
    };
    let source = meta.source_attachment.as_mut().unwrap();
    if source.installed_release.as_ref().is_some_and(|previous| {
        previous.project_id == installed.project_id
            && previous.file_id == installed.file_id
            && previous.content_digest == installed.content_digest
    }) {
        return Ok(meta);
    }
    source.installed_release = Some(installed);
    managed_storage::write_managed_mod(managed_root, &meta)?;
    Ok(meta)
}
