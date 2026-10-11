use crate::curseforge_client::SourceLookupError;
use serde::Deserialize;

pub const PAGE_SIZE: u32 = 24;
pub const MAX_SEARCH_RESULTS: u32 = 10_000;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowseSort {
    #[default]
    Popular,
    Downloads,
    Updated,
    Name,
}
impl BrowseSort {
    pub fn label(self) -> &'static str {
        match self {
            Self::Popular => "Popular",
            Self::Downloads => "Most downloaded",
            Self::Updated => "Recently updated",
            Self::Name => "Name",
        }
    }
    pub fn field(self) -> u8 {
        match self {
            Self::Popular => 2,
            Self::Downloads => 6,
            Self::Updated => 3,
            Self::Name => 4,
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
    pub index: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub game_id: Option<u32>,
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
