use chrono::Utc;

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub(crate) struct SourceType {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub(crate) struct Source {
    pub id: uuid::Uuid,
    pub name: String,
    pub url: String,
    pub type_id: i32,
    pub paywall: Option<bool>,
    pub feed_available: Option<bool>,
    pub description: Option<String>,
    pub short_name: Option<String>,
    pub state: Option<String>,
    pub city: Option<String>,
    pub create_timestamp: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub(crate) struct Feed {
    pub id: uuid::Uuid,
    pub source_id: Option<uuid::Uuid>,
    pub url: String,
    pub title: Option<String>,
    pub create_timestamp: chrono::DateTime<Utc>,
    pub feed_type: Option<String>,
    pub ttl: Option<i32>,
}


#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub(crate) struct NewsItem {
    pub id: uuid::Uuid,
    pub feed_id: uuid::Uuid,
    pub guid: String,
    pub title: String,
    pub published_timestamp: chrono::DateTime<Utc>,
    pub url: String,
    pub create_timestamp: chrono::DateTime<Utc>,
}