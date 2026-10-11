use crate::curseforge_client::SourceLookupError;
use serde::Deserialize;

pub const PAGE_SIZE: u32 = 24;
pub const MAX_SEARCH_RESULTS: u32 = 10_000;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowseSort {
    Relevance,
    #[default]
    Popular,
    Downloads,
    Updated,
    Created,
    Name,
}
impl BrowseSort {
    pub fn label(self) -> &'static str {
        match self {
            Self::Relevance => "Relevancy",
            Self::Popular => "Popularity",
            Self::Downloads => "Total Downloads",
            Self::Updated => "Latest update",
            Self::Created => "Date released",
            Self::Name => "A-Z",
        }
    }
    pub fn field(self) -> Option<u8> {
        match self {
            Self::Relevance => None,
            Self::Popular => Some(2),
            Self::Downloads => Some(6),
            Self::Updated => Some(3),
            Self::Created => Some(11),
            Self::Name => Some(4),
        }
    }
    pub fn order(self) -> &'static str {
        if self == Self::Name {
            "asc"
        } else {
            "desc"
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct BrowseQuery {
    pub search: String,
    pub sort: BrowseSort,
    pub category: Option<u64>,
    pub class_id: Option<u64>,
    pub game_version: Option<String>,
    pub index: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub game_id: Option<u32>,
    #[serde(default)]
    pub class_id: Option<u64>,
    #[serde(default)]
    pub parent_category_id: Option<u64>,
    #[serde(default)]
    pub is_class: Option<bool>,
    #[serde(default)]
    pub display_index: Option<u32>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct BrowseProject {
    pub id: u64,
    pub name: String,
    pub summary: String,
    pub authors: Vec<String>,
    pub source_url: Option<String>,
    pub preview_url: Option<String>,
    pub screenshots: Vec<String>,
    pub downloads: f64,
    pub updated: Option<String>,
    pub categories: Vec<Category>,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub index: u32,
    pub page_size: u32,
    pub result_count: u32,
    pub total_count: u32,
}
#[derive(Debug, Clone, PartialEq)]
pub struct BrowsePage {
    pub projects: Vec<BrowseProject>,
    pub pagination: Pagination,
}
impl BrowsePage {
    pub fn next_index(&self) -> Option<u32> {
        let next = self
            .pagination
            .index
            .checked_add(self.pagination.result_count)?;
        (self.pagination.result_count > 0
            && next < self.pagination.total_count.min(MAX_SEARCH_RESULTS))
        .then_some(next)
    }
}

#[derive(Deserialize)]
struct Response {
    data: Vec<ProjectDto>,
    pagination: Pagination,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectDto {
    id: u64,
    game_id: u32,
    name: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    authors: Vec<Author>,
    links: Option<Links>,
    logo: Option<Photo>,
    #[serde(default)]
    screenshots: Vec<Photo>,
    #[serde(default)]
    download_count: f64,
    date_modified: Option<String>,
    #[serde(default)]
    categories: Vec<Category>,
}
#[derive(Deserialize)]
struct Author {
    name: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Links {
    website_url: Option<String>,
}
#[derive(Deserialize)]
struct Photo {
    url: Option<String>,
}

pub fn parse_browse_response(body: &str) -> Result<BrowsePage, SourceLookupError> {
    let response: Response =
        serde_json::from_str(body).map_err(|_| SourceLookupError::InvalidResponse)?;
    if response.pagination.page_size == 0
        || response.pagination.result_count > response.pagination.page_size
    {
        return Err(SourceLookupError::InvalidResponse);
    }
    let projects = response
        .data
        .into_iter()
        .filter(|project| {
            project.game_id == crate::curseforge_client::SIMS4_GAME_ID
                && !project.name.trim().is_empty()
        })
        .map(|project| BrowseProject {
            id: project.id,
            name: project.name.trim().into(),
            summary: project.summary,
            authors: project
                .authors
                .into_iter()
                .map(|author| author.name)
                .collect(),
            source_url: project.links.and_then(|links| links.website_url),
            preview_url: project.logo.and_then(|photo| photo.url),
            screenshots: project
                .screenshots
                .into_iter()
                .filter_map(|photo| photo.url)
                .collect(),
            downloads: project.download_count,
            updated: project.date_modified,
            categories: project.categories,
        })
        .collect();
    Ok(BrowsePage {
        projects,
        pagination: response.pagination,
    })
}

pub fn parse_categories_response(body: &str) -> Result<Vec<Category>, SourceLookupError> {
    #[derive(Deserialize)]
    struct Response {
        data: Vec<Category>,
    }
    let response: Response =
        serde_json::from_str(body).map_err(|_| SourceLookupError::InvalidResponse)?;
    let mut categories: Vec<_> = response
        .data
        .into_iter()
        .filter(|category| {
            category.game_id == Some(crate::curseforge_client::SIMS4_GAME_ID)
                && !category.name.trim().is_empty()
        })
        .collect();
    categories.sort_by_cached_key(|category| category.name.to_lowercase());
    Ok(categories)
}

pub fn category_options(categories: &[Category], class_id: u64) -> Vec<(u64, String)> {
    let mut options = Vec::new();
    for category in categories
        .iter()
        .filter(|c| c.class_id == Some(class_id) && c.is_class != Some(true))
    {
        let mut path = Vec::new();
        let mut seen = Vec::new();
        let mut current = Some(category);
        while let Some(node) = current {
            if seen.contains(&node.id) {
                break;
            }
            seen.push(node.id);
            if node.id == class_id && node.is_class == Some(true) {
                path.reverse();
                options.push((category.id, path.join(" / ")));
                break;
            }
            path.push(node.name.trim());
            current = node
                .parent_category_id
                .and_then(|id| categories.iter().find(|c| c.id == id));
        }
    }
    options.sort_by_cached_key(|(_, label)| label.to_lowercase());
    options
}

pub fn parse_game_versions_response(body: &str) -> Result<Vec<String>, SourceLookupError> {
    #[derive(Deserialize)]
    struct Group {
        versions: Vec<String>,
    }
    #[derive(Deserialize)]
    struct Response {
        data: Vec<Group>,
    }
    let response: Response =
        serde_json::from_str(body).map_err(|_| SourceLookupError::InvalidResponse)?;
    let mut versions: Vec<_> = response
        .data
        .into_iter()
        .flat_map(|group| group.versions)
        .filter_map(|name| {
            let parts: Option<Vec<u32>> = name.split('.').map(|part| part.parse().ok()).collect();
            parts
                .filter(|parts| parts.len() == 3)
                .map(|parts| (parts, name))
        })
        .collect();
    versions.sort_by(|a, b| b.0.cmp(&a.0));
    versions.dedup_by(|a, b| a.1 == b.1);
    Ok(versions.into_iter().map(|(_, name)| name).collect())
}
