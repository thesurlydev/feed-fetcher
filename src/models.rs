#[derive(Debug)]
pub(crate) struct SourceType {
    pub id: i32,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Source {
    pub uuid: uuid::Uuid,
    pub name: String,
    pub url: String,
    pub type_id: i32,
    pub paywall: bool,
    pub feed_available: bool,
    pub description: Option<String>,
    pub short_name: Option<String>,
    pub state: Option<String>,
    pub city: Option<String>,
    pub create_timestamp: chrono::NaiveDateTime,
}