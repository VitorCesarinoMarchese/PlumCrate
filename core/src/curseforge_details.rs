//! Read-only author documents and provider project identities. HTML never executes.
use crate::curseforge_client::{
    classify_status, CurseForgeClient, CurseForgeTransport, SourceLookupError, SIMS4_GAME_ID,
};
use scraper::{ElementRef, Html, Node};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SafeUrl(String);
impl SafeUrl {
    pub fn parse(value: &str) -> Option<Self> {
        let url = url::Url::parse(value).ok()?;
        (url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && !value.chars().any(char::is_control))
        .then(|| Self(url.to_string()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn project_slug(&self) -> Option<String> {
        let url = url::Url::parse(&self.0).ok()?;
        if !matches!(
            url.host_str(),
            Some("curseforge.com" | "www.curseforge.com")
        ) || url.port().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return None;
        }
        let segments = url
            .path()
            .trim_end_matches('/')
            .split('/')
            .collect::<Vec<_>>();
        if segments.len() != 4
            || segments[1] != "sims4"
            || !matches!(
                segments[2],
                "mods"
                    | "build-buy"
                    | "create-a-sim"
                    | "rooms-lots"
                    | "sims-households"
                    | "pets"
                    | "save-files"
                    | "worlds"
            )
            || segments[3].is_empty()
            || !segments[3]
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return None;
        }
        Some(segments[3].into())
    }
    fn canonical_project(&self) -> Option<String> {
        self.project_slug()?;
        let url = url::Url::parse(&self.0).ok()?;
        Some(url.path().trim_end_matches('/').to_owned())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorLink {
    pub label: String,
    pub url: SafeUrl,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph,
    Heading(u8),
    ListItem,
    Quote,
    Preformatted,
    TableRow,
    Rule,
    Image,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescriptionBlock {
    pub kind: BlockKind,
    pub text: String,
    pub links: Vec<AuthorLink>,
    pub image: Option<SafeUrl>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DescriptionDocument {
    pub blocks: Vec<DescriptionBlock>,
}
impl DescriptionDocument {
    pub fn author_projects(&self) -> Vec<AuthorLink> {
        let mut links = Vec::new();
        for link in self.blocks.iter().flat_map(|b| &b.links) {
            if link.url.project_slug().is_some()
                && !links.iter().any(|l: &AuthorLink| l.url == link.url)
            {
                links.push(link.clone());
            }
        }
        links
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectReference {
    pub project_id: u64,
    pub title: String,
    pub source_url: SafeUrl,
}
fn response<T: CurseForgeTransport>(
    client: &CurseForgeClient<T>,
    path: &str,
) -> Result<String, SourceLookupError> {
    let (status, body) = client.get_update_response(path)?;
    classify_status(status)?;
    if body.len() > 2 * 1024 * 1024 {
        return Err(SourceLookupError::InvalidResponse);
    }
    Ok(body)
}
pub fn load_description<T: CurseForgeTransport>(
    client: &CurseForgeClient<T>,
    project_id: u64,
    project_url: &str,
) -> Result<DescriptionDocument, SourceLookupError> {
    if project_id == 0 {
        return Err(SourceLookupError::InvalidResponse);
    }
    parse_description_response(
        &response(client, &format!("mods/{project_id}/description"))?,
        project_url,
    )
}
pub fn load_project_reference<T: CurseForgeTransport>(
    client: &CurseForgeClient<T>,
    project_id: u64,
) -> Result<ProjectReference, SourceLookupError> {
    let value: Value = serde_json::from_str(&response(client, &format!("mods/{project_id}"))?)
        .map_err(|_| SourceLookupError::InvalidResponse)?;
    let data = &value["data"];
    let title = data["name"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or(SourceLookupError::InvalidResponse)?;
    let source_url = data["links"]["websiteUrl"]
        .as_str()
        .and_then(SafeUrl::parse)
        .filter(|u| u.project_slug().is_some())
        .ok_or(SourceLookupError::InvalidResponse)?;
    if project_id == 0
        || data["id"].as_u64() != Some(project_id)
        || data["gameId"].as_u64() != Some(u64::from(SIMS4_GAME_ID))
    {
        return Err(SourceLookupError::InvalidResponse);
    }
    Ok(ProjectReference {
        project_id,
        title: title.into(),
        source_url,
    })
}
pub fn resolve_author_project<T: CurseForgeTransport>(
    client: &CurseForgeClient<T>,
    link: &SafeUrl,
) -> Result<ProjectReference, SourceLookupError> {
    let slug = link
        .project_slug()
        .ok_or(SourceLookupError::InvalidResponse)?;
    let matches = client
        .search_mods_by_slug(&slug)?
        .into_iter()
        .filter(|p| {
            p.source_url
                .as_deref()
                .and_then(SafeUrl::parse)
                .is_some_and(|u| u.canonical_project() == link.canonical_project())
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(SourceLookupError::NoUsableEvidence);
    }
    let reference = load_project_reference(client, matches[0].project_id)?;
    if reference.source_url.canonical_project() != link.canonical_project() {
        return Err(SourceLookupError::NoUsableEvidence);
    }
    Ok(reference)
}

pub fn parse_description_response(
    body: &str,
    project_url: &str,
) -> Result<DescriptionDocument, SourceLookupError> {
    if body.len() > 2 * 1024 * 1024 {
        return Err(SourceLookupError::InvalidResponse);
    }
    let value: Value =
        serde_json::from_str(body).map_err(|_| SourceLookupError::InvalidResponse)?;
    let html = value["data"]
        .as_str()
        .filter(|s| s.len() <= 1024 * 1024)
        .ok_or(SourceLookupError::InvalidResponse)?;
    let base = SafeUrl::parse(project_url).ok_or(SourceLookupError::InvalidResponse)?;
    let document = Html::parse_fragment(html);
    let mut builder = Builder {
        document: DescriptionDocument::default(),
        text: String::new(),
        links: vec![],
        kind: BlockKind::Paragraph,
        base,
        visited: 0,
        lists: vec![],
    };
    builder.walk(document.root_element(), 0)?;
    builder.flush();
    Ok(builder.document)
}
struct Builder {
    document: DescriptionDocument,
    text: String,
    links: Vec<AuthorLink>,
    kind: BlockKind,
    base: SafeUrl,
    visited: usize,
    lists: Vec<Option<usize>>,
}
impl Builder {
    fn flush(&mut self) {
        let text = std::mem::take(&mut self.text);
        let text = if self.kind == BlockKind::Preformatted {
            text.trim().to_owned()
        } else {
            text.split_whitespace().collect::<Vec<_>>().join(" ")
        };
        let links = std::mem::take(&mut self.links);
        if !text.is_empty() || !links.is_empty() {
            self.document.blocks.push(DescriptionBlock {
                kind: self.kind,
                text,
                links,
                image: None,
            });
        }
    }
    fn url(&self, value: &str) -> Option<SafeUrl> {
        let base = url::Url::parse(self.base.as_str()).ok()?;
        SafeUrl::parse(base.join(value).ok()?.as_str())
    }
    fn walk(&mut self, element: ElementRef<'_>, depth: usize) -> Result<(), SourceLookupError> {
        self.visited += 1;
        if depth > 64 || self.visited > 20_000 || self.document.blocks.len() > 10_000 {
            return Err(SourceLookupError::InvalidResponse);
        }
        let name = element.value().name();
        if matches!(
            name,
            "script"
                | "style"
                | "iframe"
                | "object"
                | "embed"
                | "form"
                | "noscript"
                | "template"
                | "head"
        ) || element.value().attr("hidden").is_some()
        {
            return Ok(());
        }
        let old = self.kind;
        let in_table = old == BlockKind::TableRow;
        let boundary = !in_table
            && matches!(
                name,
                "p" | "div"
                    | "section"
                    | "article"
                    | "h1"
                    | "h2"
                    | "h3"
                    | "h4"
                    | "h5"
                    | "h6"
                    | "li"
                    | "blockquote"
                    | "pre"
                    | "tr"
                    | "ul"
                    | "ol"
            );
        if boundary {
            self.flush();
        }
        self.kind = match name {
            "h1" => BlockKind::Heading(1),
            "h2" => BlockKind::Heading(2),
            "h3" => BlockKind::Heading(3),
            "h4" => BlockKind::Heading(4),
            "h5" => BlockKind::Heading(5),
            "h6" => BlockKind::Heading(6),
            "li" => BlockKind::ListItem,
            "blockquote" => BlockKind::Quote,
            "pre" => BlockKind::Preformatted,
            "tr" => BlockKind::TableRow,
            _ => old,
        };
        if matches!(name, "td" | "th") && !self.text.trim().is_empty() {
            self.text.push_str(" | ");
        }
        if matches!(name, "ul" | "ol") {
            self.lists.push((name == "ol").then_some(0));
        }
        if name == "li" {
            match self.lists.last_mut() {
                Some(Some(count)) => {
                    *count += 1;
                    self.text.push_str(&format!("{count}. "));
                }
                _ => self.text.push_str("• "),
            }
        }
        if name == "br" {
            self.text.push('\n');
            self.flush();
        }
        if name == "hr" {
            self.flush();
            self.document.blocks.push(DescriptionBlock {
                kind: BlockKind::Rule,
                text: String::new(),
                links: vec![],
                image: None,
            });
        }
        if name == "img" {
            self.flush();
            let text = element.value().attr("alt").unwrap_or_default().to_owned();
            let image = element.value().attr("src").and_then(|u| self.url(u));
            if image.is_some() || !text.is_empty() {
                self.document.blocks.push(DescriptionBlock {
                    kind: BlockKind::Image,
                    text,
                    links: vec![],
                    image,
                });
            }
        }
        let start = self.text.len();
        for child in element.children() {
            match child.value() {
                Node::Text(text) => self.text.push_str(text),
                Node::Element(_) => {
                    if let Some(element) = ElementRef::wrap(child) {
                        self.walk(element, depth + 1)?;
                    }
                }
                _ => {}
            }
        }
        if name == "a" {
            if let Some(url) = element.value().attr("href").and_then(|u| self.url(u)) {
                let label = self
                    .text
                    .get(start..)
                    .unwrap_or_default()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                self.links.push(AuthorLink {
                    label: if label.is_empty() {
                        url.as_str().into()
                    } else {
                        label
                    },
                    url,
                });
            }
        }
        if boundary {
            self.flush();
        }
        if matches!(name, "ul" | "ol") {
            self.lists.pop();
        }
        self.kind = old;
        Ok(())
    }
}
