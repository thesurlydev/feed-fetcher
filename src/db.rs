use std::env;
use sqlx::{PgPool, Pool, Postgres, query_as};
use crate::models::{Feed, NewsItem, Source, SourceType};

pub(crate) async fn source_types() -> Result<Vec<SourceType>, sqlx::Error> {
    let pool: Pool<Postgres> = get_pool().await;
    query_as!(SourceType, "select * from source_type")
        .fetch_all(&pool)
        .await
}

pub(crate) async fn sources() -> Result<Vec<Source>, sqlx::Error> {
    let pool: Pool<Postgres> = get_pool().await;
    query_as!(Source, r#"SELECT * FROM source"#)
        .fetch_all(&pool)
        .await
}

pub(crate) async fn source_type_by_name(name: &str) -> Result<SourceType, sqlx::Error> {
    let pool: Pool<Postgres> = get_pool().await;
    query_as!(SourceType, r#"SELECT * FROM source_type WHERE name = $1"#, name)
        .fetch_one(&pool)
        .await
}

pub(crate) async fn feeds() -> Result<Vec<Feed>, sqlx::Error> {
    let pool: Pool<Postgres> = get_pool().await;
    query_as!(Feed, r#"SELECT * FROM feed"#)
        .fetch_all(&pool)
        .await
}

pub(crate) async fn news() -> Result<Vec<NewsItem>, sqlx::Error> {
    let pool: Pool<Postgres> = get_pool().await;
    query_as!(NewsItem, r#"SELECT * FROM news"#)
        .fetch_all(&pool)
        .await
}

pub(crate) async fn save_source_type(source_type: &SourceType) -> anyhow::Result<i32> {
    let pool: Pool<Postgres> = get_pool().await;
    let rec = sqlx::query!("INSERT INTO source_type (id, name) VALUES ($1, $2) RETURNING id", source_type.id, source_type.name)
        .fetch_one(&pool)
        .await?;
    Ok(rec.id)
}

async fn get_pool() -> Pool<Postgres> {
    let db_url = &env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool: Pool<Postgres> = PgPool::connect(db_url.as_str()).await.expect("Failed to connect to Postgres");
    return pool;
}

pub(crate) async fn save_source(source: &Source) -> anyhow::Result<uuid::Uuid> {
    let pool: Pool<Postgres> = get_pool().await;
    let rec = sqlx::query!(r#"
INSERT INTO source (id, name, url, type_id, paywall, feed_available, description, short_name, state, city, create_timestamp)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
RETURNING id
"#,
        source.id, source.name, source.url, source.type_id, source.paywall, source.feed_available, source.description, source.short_name, source.state, source.city, source.create_timestamp)
        .fetch_one(&pool)
        .await?;
    Ok(rec.id)
}

pub(crate) async fn save_feed(feed: &Feed) -> anyhow::Result<uuid::Uuid> {
    let pool: Pool<Postgres> = get_pool().await;
    let rec = sqlx::query!("INSERT INTO feed (id, url, title, source_id, feed_type) VALUES ($1, $2, $3, $4, $5) RETURNING id", feed.id, feed.url, feed.title, feed.source_id, feed.feed_type)
        .fetch_one(&pool)
        .await?;
    Ok(rec.id)
}

pub(crate) async fn save_news_item(ni: &NewsItem) -> anyhow::Result<uuid::Uuid> {
    let pool: Pool<Postgres> = get_pool().await;
    let rec = sqlx::query!(
        r#"INSERT INTO news
        (id, title, url, published_timestamp, guid, feed_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id"#,
        ni.id, ni.title, ni.url, ni.published_timestamp, ni.guid, ni.feed_id)
        .fetch_one(&pool)
        .await?;
    Ok(rec.id)
}