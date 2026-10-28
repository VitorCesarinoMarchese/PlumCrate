use serde::Deserialize;

pub const CURSEFORGE_API_BASE: &str = "https://api.curseforge.com";
pub const SIMS4_GAME_ID: u32 = 78062;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceLookupError {
    MissingApiKey,
    Unauthorized,
    RateLimited,
    Network,
    InvalidResponse,
    ProviderUnavailable,
    NoUsableEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurseForgeRequest {
    pub url: String,
    pub api_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurseForgeModSummary {
    pub project_id: u64,
    pub name: String,
    pub slug: Option<String>,
    pub source_url: Option<String>,
    pub preview_url: Option<String>,
    pub authors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurseForgeFileSummary {
    pub file_id: u64,
    pub project_id: u64,
    pub display_name: Option<String>,
    pub file_name: String,
    pub game_versions: Vec<String>,
    pub hashes: Vec<CurseForgeHash>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurseForgeHash {
    pub algo: u32,
    pub value: String,
}

pub trait CurseForgeTransport {
    fn get(&self, request: &CurseForgeRequest) -> Result<(u16, String), SourceLookupError>;
}

pub struct UreqCurseForgeTransport {
    deadline: std::time::Instant,
}

impl Default for UreqCurseForgeTransport {
    fn default() -> Self {
        Self::with_budget(std::time::Duration::from_secs(20))
    }
}
impl UreqCurseForgeTransport {
    pub fn with_budget(budget: std::time::Duration) -> Self {
        Self { deadline: std::time::Instant::now() + budget }
    }
}

impl CurseForgeTransport for UreqCurseForgeTransport {
    fn get(&self, request: &CurseForgeRequest) -> Result<(u16, String), SourceLookupError> {
        let remaining = self.deadline.checked_duration_since(std::time::Instant::now())
            .filter(|duration| !duration.is_zero()).ok_or(SourceLookupError::Network)?;
        let timeout = remaining.min(std::time::Duration::from_secs(5));
        let agent = ureq::builder().timeout(timeout).timeout_connect(timeout).build();
        let response = agent.get(&request.url)
            .set("x-api-key", &request.api_key)
            .set("User-Agent", "PlumCrate/0.1 source lookup")
            .call();

        match response {
            Ok(response) => {
                let status = response.status();
                let body = response.into_string().map_err(|_| SourceLookupError::InvalidResponse)?;
                Ok((status, body))
            }
            Err(ureq::Error::Status(status, response)) => {
                let body = response.into_string().unwrap_or_default();
                Ok((status, body))
            }
            Err(_) => Err(SourceLookupError::Network),
        }
    }
}

pub struct CurseForgeClient<T> {
    api_key: String,
    transport: T,
}

impl<T: CurseForgeTransport> CurseForgeClient<T> {
    pub fn new(api_key: Option<&str>, transport: T) -> Result<Self, SourceLookupError> {
        let key = api_key.map(str::trim).filter(|key| !key.is_empty()).ok_or(SourceLookupError::MissingApiKey)?;
        Ok(Self { api_key: key.to_string(), transport })
    }

    pub fn browse_mods(&self, query: &crate::curseforge_browse::BrowseQuery) -> Result<crate::curseforge_browse::BrowsePage, SourceLookupError> {
        use crate::curseforge_browse::{MAX_SEARCH_RESULTS, PAGE_SIZE};
        if query.index >= MAX_SEARCH_RESULTS { return Err(SourceLookupError::InvalidResponse); }
        let mut request = build_search_request(Some(&self.api_key), &query.search)?;
        if let Some(field) = query.sort.field() {
            request.url.push_str(&format!("&sortField={field}&sortOrder={}", query.sort.order()));
        }
        request.url.push_str(&format!("&index={}&pageSize={PAGE_SIZE}", query.index));
        if let Some(class_id) = query.class_id { request.url.push_str(&format!("&classId={class_id}")); }
        if let Some(category) = query.category { request.url.push_str(&format!("&categoryId={category}")); }
        if let Some(version) = &query.game_version { request.url.push_str(&format!("&gameVersion={}", encode_query(version))); }
        let (status, body) = self.transport.get(&request)?;
        classify_status(status)?;
        let page = crate::curseforge_browse::parse_browse_response(&body)?;
        if page.pagination.index != query.index { return Err(SourceLookupError::InvalidResponse); }
        Ok(page)
    }

    pub fn browse_categories(&self) -> Result<Vec<crate::curseforge_browse::Category>, SourceLookupError> {
        let request = CurseForgeRequest { url: format!("{CURSEFORGE_API_BASE}/v1/categories?gameId={SIMS4_GAME_ID}"), api_key: self.api_key.clone() };
        let (status, body) = self.transport.get(&request)?;
        classify_status(status)?;
        crate::curseforge_browse::parse_categories_response(&body)
    }

    pub fn browse_game_versions(&self) -> Result<Vec<String>, SourceLookupError> {
        let request = CurseForgeRequest { url: format!("{CURSEFORGE_API_BASE}/v1/games/{SIMS4_GAME_ID}/versions"), api_key: self.api_key.clone() };
        let (status, body) = self.transport.get(&request)?;
        classify_status(status)?;
        crate::curseforge_browse::parse_game_versions_response(&body)
    }

    pub fn search_mods(&self, search_filter: &str) -> Result<Vec<CurseForgeModSummary>, SourceLookupError> {
        let request = build_search_request(Some(&self.api_key), search_filter)?;
        let (status, body) = self.transport.get(&request)?;
        classify_status(status)?;
        parse_search_response(&body)
    }

    pub fn search_mods_by_slug(&self, slug: &str) -> Result<Vec<CurseForgeModSummary>, SourceLookupError> {
        let request = build_slug_search_request(Some(&self.api_key), slug)?;
        let (status, body) = self.transport.get(&request)?;
        classify_status(status)?;
        parse_search_response(&body)
    }

    pub fn get_mod_files(&self, project_id: u64) -> Result<Vec<CurseForgeFileSummary>, SourceLookupError> {
        let request = build_files_request(Some(&self.api_key), project_id)?;
        let (status, body) = self.transport.get(&request)?;
        classify_status(status)?;
        parse_files_response(&body)
    }

    pub(crate) fn get_update_response(&self, path: &str) -> Result<(u16, String), SourceLookupError> {
        self.transport.get(&CurseForgeRequest {
            url: format!("{CURSEFORGE_API_BASE}/v1/{path}"),
            api_key: self.api_key.clone(),
        })
    }
}

pub fn build_search_request(api_key: Option<&str>, search_filter: &str) -> Result<CurseForgeRequest, SourceLookupError> {
    let key = api_key.map(str::trim).filter(|key| !key.is_empty()).ok_or(SourceLookupError::MissingApiKey)?;
    let query = encode_query(search_filter.trim());
    Ok(CurseForgeRequest {
        url: format!("{CURSEFORGE_API_BASE}/v1/mods/search?gameId={SIMS4_GAME_ID}&searchFilter={query}"),
        api_key: key.to_string(),
    })
}

pub fn build_slug_search_request(api_key: Option<&str>, slug: &str) -> Result<CurseForgeRequest, SourceLookupError> {
    let key = api_key.map(str::trim).filter(|key| !key.is_empty()).ok_or(SourceLookupError::MissingApiKey)?;
    let slug = encode_query(slug.trim());
    Ok(CurseForgeRequest {
        url: format!("{CURSEFORGE_API_BASE}/v1/mods/search?gameId={SIMS4_GAME_ID}&slug={slug}"),
        api_key: key.to_string(),
    })
}

pub fn build_files_request(api_key: Option<&str>, project_id: u64) -> Result<CurseForgeRequest, SourceLookupError> {
    let key = api_key.map(str::trim).filter(|key| !key.is_empty()).ok_or(SourceLookupError::MissingApiKey)?;
    Ok(CurseForgeRequest {
        url: format!("{CURSEFORGE_API_BASE}/v1/mods/{project_id}/files"),
        api_key: key.to_string(),
    })
}

pub fn classify_status(status: u16) -> Result<(), SourceLookupError> {
    match status {
        200..=299 => Ok(()),
        401 | 403 => Err(SourceLookupError::Unauthorized),
        429 => Err(SourceLookupError::RateLimited),
        500..=599 => Err(SourceLookupError::ProviderUnavailable),
        _ => Err(SourceLookupError::Network),
    }
}

pub fn parse_search_response(json: &str) -> Result<Vec<CurseForgeModSummary>, SourceLookupError> {
    let parsed: SearchResponse = serde_json::from_str(json).map_err(|_| SourceLookupError::InvalidResponse)?;
    Ok(parsed
        .data
        .into_iter()
        .filter_map(|mod_dto| {
            let project_id = mod_dto.id?;
            let name = mod_dto.name?.trim().to_string();
            (!name.is_empty()).then(|| CurseForgeModSummary {
                project_id,
                name,
                slug: mod_dto.slug.filter(|slug| !slug.trim().is_empty()),
                source_url: mod_dto.links.and_then(|links| links.website_url).filter(|url| !url.trim().is_empty()),
                preview_url: mod_dto.logo.and_then(|logo| logo.url).filter(|url| !url.trim().is_empty()),
                authors: mod_dto
                    .authors
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|author| author.name.map(|name| name.trim().to_string()).filter(|name| !name.is_empty()))
                    .collect(),
            })
        })
        .collect())
}

pub fn parse_files_response(json: &str) -> Result<Vec<CurseForgeFileSummary>, SourceLookupError> {
    let parsed: FilesResponse = serde_json::from_str(json).map_err(|_| SourceLookupError::InvalidResponse)?;
    Ok(parsed
        .data
        .into_iter()
        .filter_map(|file| {
            let file_id = file.id?;
            let project_id = file.mod_id?;
            let file_name = file.file_name?.trim().to_string();
            (!file_name.is_empty()).then(|| CurseForgeFileSummary {
                file_id,
                project_id,
                display_name: file.display_name.filter(|name| !name.trim().is_empty()),
                file_name,
                game_versions: file.game_versions.unwrap_or_default(),
                hashes: file
                    .hashes
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|hash| Some(CurseForgeHash { algo: hash.algo?, value: hash.value? }))
                    .collect(),
            })
        })
        .collect())
}

pub fn has_usable_file_metadata(files: &[CurseForgeFileSummary]) -> bool {
    files.iter().any(|file| !file.file_name.trim().is_empty() || !file.hashes.is_empty())
}

fn encode_query(raw: &str) -> String {
    raw.bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => vec![byte as char],
            b' ' => vec!['+'],
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchResponse {
    data: Vec<ModDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModDto {
    id: Option<u64>,
    name: Option<String>,
    slug: Option<String>,
    links: Option<LinksDto>,
    logo: Option<LogoDto>,
    authors: Option<Vec<AuthorDto>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinksDto {
    website_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogoDto {
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuthorDto {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FilesResponse {
    data: Vec<FileDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileDto {
    id: Option<u64>,
    mod_id: Option<u64>,
    display_name: Option<String>,
    file_name: Option<String>,
    game_versions: Option<Vec<String>>,
    hashes: Option<Vec<HashDto>>,
}

#[derive(Debug, Deserialize)]
struct HashDto {
    algo: Option<u32>,
    value: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEARCH_FIXTURE: &str = r#"
    {
      "data": [
        {
          "id": 551680,
          "name": "MC Command Center",
          "slug": "mc-command-center",
          "links": { "websiteUrl": "https://www.curseforge.com/sims4/mods/mc-command-center" },
          "logo": { "url": "https://media.forgecdn.net/avatars/cover.png" },
          "authors": [{ "name": "Deaderpool" }]
        }
      ]
    }
    "#;

    const FILES_FIXTURE: &str = r#"
    {
      "data": [
        {
          "id": 67890,
          "modId": 551680,
          "displayName": "McCmdCenter_AllModules_2026_2_0.zip",
          "fileName": "McCmdCenter_AllModules_2026_2_0.zip",
          "gameVersions": ["1.110", "2026.2.0"],
          "hashes": [
            { "algo": 1, "value": "123456789" },
            { "algo": 2, "value": "abcdef" }
          ]
        }
      ]
    }
    "#;

    #[test]
    fn transport_deadline_bounds_stalled_response() {
        use std::{io::Read, net::TcpListener, time::{Duration, Instant}};
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = server.local_addr().unwrap();
        let listener = std::thread::spawn(move || {
            let (mut socket, _) = server.accept().unwrap();
            let mut bytes = [0; 1024];
            socket.read(&mut bytes).unwrap();
            std::thread::sleep(Duration::from_millis(300));
        });
        let transport = UreqCurseForgeTransport::with_budget(Duration::from_millis(50));
        let start = Instant::now();
        assert_eq!(transport.get(&CurseForgeRequest {url: format!("http://{address}"), api_key: "fixture".into()}), Err(SourceLookupError::Network));
        assert!(start.elapsed() < Duration::from_millis(250));
        // The second request must not receive a fresh budget.
        assert_eq!(transport.get(&CurseForgeRequest {url: format!("http://{address}"), api_key: "fixture".into()}), Err(SourceLookupError::Network));
        listener.join().unwrap();
    }

    #[test]
    fn sims4_game_id_is_stable() {
        assert_eq!(SIMS4_GAME_ID, 78062);
    }

    #[test]
    fn api_key_is_required_for_requests() {
        assert_eq!(build_search_request(None, "mccc"), Err(SourceLookupError::MissingApiKey));
        assert_eq!(build_files_request(Some(""), 551680), Err(SourceLookupError::MissingApiKey));
    }

    #[test]
    fn builds_search_and_files_requests() {
        let search = build_search_request(Some("key"), "mc command center").expect("request");
        assert_eq!(search.api_key, "key");
        assert_eq!(search.url, "https://api.curseforge.com/v1/mods/search?gameId=78062&searchFilter=mc+command+center");

        let slug = build_slug_search_request(Some("key"), "mc-command-center").expect("request");
        assert_eq!(slug.url, "https://api.curseforge.com/v1/mods/search?gameId=78062&slug=mc-command-center");

        let files = build_files_request(Some("key"), 551680).expect("request");
        assert_eq!(files.url, "https://api.curseforge.com/v1/mods/551680/files");
    }

    #[test]
    fn classifies_provider_status_codes() {
        assert_eq!(classify_status(200), Ok(()));
        assert_eq!(classify_status(401), Err(SourceLookupError::Unauthorized));
        assert_eq!(classify_status(403), Err(SourceLookupError::Unauthorized));
        assert_eq!(classify_status(429), Err(SourceLookupError::RateLimited));
        assert_eq!(classify_status(503), Err(SourceLookupError::ProviderUnavailable));
    }

    #[test]
    fn parses_search_response_fixture() {
        let mods = parse_search_response(SEARCH_FIXTURE).expect("parse");
        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0].project_id, 551680);
        assert_eq!(mods[0].name, "MC Command Center");
        assert_eq!(mods[0].slug.as_deref(), Some("mc-command-center"));
        assert_eq!(mods[0].source_url.as_deref(), Some("https://www.curseforge.com/sims4/mods/mc-command-center"));
        assert_eq!(mods[0].preview_url.as_deref(), Some("https://media.forgecdn.net/avatars/cover.png"));
        assert_eq!(mods[0].authors, vec!["Deaderpool"]);
    }

    #[test]
    fn parses_file_metadata_fixture() {
        let files = parse_files_response(FILES_FIXTURE).expect("parse");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_id, 67890);
        assert_eq!(files[0].project_id, 551680);
        assert_eq!(files[0].file_name, "McCmdCenter_AllModules_2026_2_0.zip");
        assert_eq!(files[0].game_versions, vec!["1.110", "2026.2.0"]);
        assert_eq!(files[0].hashes[1].value, "abcdef");
        assert!(has_usable_file_metadata(&files));
    }

    struct MockTransport {
        status: u16,
        body: &'static str,
    }

    impl CurseForgeTransport for MockTransport {
        fn get(&self, _request: &CurseForgeRequest) -> Result<(u16, String), SourceLookupError> {
            Ok((self.status, self.body.to_string()))
        }
    }

    #[test]
    fn client_searches_mods_with_mocked_transport() {
        let client = CurseForgeClient::new(Some("key"), MockTransport { status: 200, body: SEARCH_FIXTURE }).expect("client");

        let mods = client.search_mods("mc command center").expect("mods");

        assert_eq!(mods[0].name, "MC Command Center");
        assert_eq!(mods[0].source_url.as_deref(), Some("https://www.curseforge.com/sims4/mods/mc-command-center"));
    }

    #[test]
    fn client_searches_mods_by_slug_with_mocked_transport() {
        let client = CurseForgeClient::new(Some("key"), MockTransport { status: 200, body: SEARCH_FIXTURE }).expect("client");

        let mods = client.search_mods_by_slug("mc-command-center").expect("mods");

        assert_eq!(mods[0].slug.as_deref(), Some("mc-command-center"));
    }

    #[test]
    fn client_fetches_files_with_mocked_transport() {
        let client = CurseForgeClient::new(Some("key"), MockTransport { status: 200, body: FILES_FIXTURE }).expect("client");

        let files = client.get_mod_files(551680).expect("files");

        assert_eq!(files[0].file_name, "McCmdCenter_AllModules_2026_2_0.zip");
        assert!(has_usable_file_metadata(&files));
    }

    #[test]
    fn client_maps_status_errors() {
        let client = CurseForgeClient::new(Some("key"), MockTransport { status: 429, body: "{}" }).expect("client");

        assert_eq!(client.search_mods("mccc"), Err(SourceLookupError::RateLimited));
    }

    #[test]
    fn malformed_provider_response_is_typed_error() {
        assert_eq!(parse_search_response("not json"), Err(SourceLookupError::InvalidResponse));
        assert_eq!(parse_files_response("{}"), Err(SourceLookupError::InvalidResponse));
    }

    #[test]
    #[ignore = "requires CURSEFORGE_API_KEY in environment"]
    fn live_search_mccc_returns_results() {
        let api_key = std::env::var("CURSEFORGE_API_KEY").expect("CURSEFORGE_API_KEY env var");
        let client = CurseForgeClient::new(Some(&api_key), UreqCurseForgeTransport::default()).expect("client");

        let mods = client.search_mods_by_slug("mc-command-center").expect("search");

        eprintln!("live CurseForge results: {mods:#?}");
        assert!(mods.iter().any(|candidate| {
            candidate.slug.as_deref() == Some("mc-command-center")
                || candidate.name.to_lowercase().contains("command center")
        }));
    }

    #[test]
    #[ignore = "requires CURSEFORGE_API_KEY in environment"]
    fn live_diagnose_mccc_search_variants() {
        let api_key = std::env::var("CURSEFORGE_API_KEY").expect("CURSEFORGE_API_KEY env var");
        let transport = UreqCurseForgeTransport::default();
        let urls = [
            format!("{CURSEFORGE_API_BASE}/v1/mods/search?gameId={SIMS4_GAME_ID}&searchFilter=mc+command+center"),
            format!("{CURSEFORGE_API_BASE}/v1/mods/search?gameId={SIMS4_GAME_ID}&searchFilter=mccc"),
            format!("{CURSEFORGE_API_BASE}/v1/mods/search?gameId={SIMS4_GAME_ID}&slug=mc-command-center"),
            format!("{CURSEFORGE_API_BASE}/v1/mods/search?gameId={SIMS4_GAME_ID}&searchFilter=mc+cmd+center"),
            format!("{CURSEFORGE_API_BASE}/v1/mods/search?searchFilter=mc+command+center"),
            format!("{CURSEFORGE_API_BASE}/v1/mods/551680"),
            format!("{CURSEFORGE_API_BASE}/v1/games/{SIMS4_GAME_ID}"),
        ];

        for url in urls {
            let request = CurseForgeRequest { url: url.clone(), api_key: api_key.clone() };
            match transport.get(&request) {
                Ok((status, body)) => {
                    let preview = body.chars().take(2000).collect::<String>();
                    eprintln!("\nURL: {url}\nSTATUS: {status}\nBODY: {preview}\n");
                }
                Err(err) => eprintln!("\nURL: {url}\nERROR: {err:?}\n"),
            }
        }
    }
}
