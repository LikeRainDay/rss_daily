use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub stars: u32,
    pub forks: u32,
    pub language: Option<String>,
    pub topics: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub pushed_at: DateTime<Utc>,
    pub open_issues: u32,
    pub owner: Owner,
    pub readme: Option<String>,
    pub stars_today: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Owner {
    pub login: String,
    pub avatar_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub content: String,
    pub language: String,
    pub key_points: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssItem {
    pub title: String,
    pub link: String,
    pub description: String,
    pub pub_date: DateTime<Utc>,
    pub image_url: String,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubSearchResponse {
    pub total_count: u32,
    pub items: Vec<GitHubRepoItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRepoItem {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub html_url: String,
    pub stargazers_count: u32,
    pub forks_count: u32,
    pub language: Option<String>,
    pub topics: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub pushed_at: String,
    pub open_issues_count: u32,
    pub owner: GitHubOwner,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubOwner {
    pub login: String,
    pub avatar_url: String,
}

impl From<GitHubRepoItem> for Repository {
    fn from(item: GitHubRepoItem) -> Self {
        Self {
            id: item.id,
            name: item.name,
            full_name: item.full_name,
            description: item.description,
            html_url: item.html_url,
            stars: item.stargazers_count,
            forks: item.forks_count,
            language: item.language,
            topics: item.topics,
            created_at: DateTime::parse_from_rfc3339(&item.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&item.updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            pushed_at: DateTime::parse_from_rfc3339(&item.pushed_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            open_issues: item.open_issues_count,
            owner: Owner {
                login: item.owner.login,
                avatar_url: item.owner.avatar_url,
            },
            readme: None,
            stars_today: None,
        }
    }
}
