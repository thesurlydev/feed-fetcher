use std::env;
use sqlx::{PgPool, Pool, Postgres, query_as};
use crate::models::{Feed, NewsItem, Source, SourceType};

pub(crate) async fn source_types() -> Result<Vec<SourceType>, sqlx::Error> {
    let pool: Pool<Postgres> = get_pool().await;
    query_as!(SourceType, "SELECT * FROM source_type")
        .fetch_all(&pool)
        .await
}

pub(crate) async fn sources() -> Result<Vec<Source>, sqlx::Error> {
    let pool: Pool<Postgres> = get_pool().await;
    query_as!(Source, r#"SELECT * FROM source"#)
        .fetch_all(&pool)
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

/*
pub(crate) async fn upsert_feed(feed: Feed) -> Result<Feed, sqlx::Error> {
    let pool: Pool<Postgres> = PgPool::connect(&env::var("DATABASE_URL").unwrap()).await?;

    query_as!(Feed, r#"
        INSERT INTO feed (id, url, title)
        VALUES ($1, $2, $3)
        RETURNING *
        "#, feed.id, feed.url, feed.title).await
}*/

/*pub(crate) async fn upsert_source(src: Source) -> Result<Source, sqlx::Error> {
    let pool: Pool<Postgres> = get_pool().await;

    query_as!(Source, r#"
        INSERT INTO source (id, name, url, type_id, paywall, feed_available, description, short_name, state, city, create_timestamp)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING *
        "#, src.id, src.name, src.url, src.type_id, src.paywall, src.feed_available, src.description, src.short_name, src.state, src.city, src.create_timestamp);
}*/

async fn get_pool() -> Pool<Postgres> {
    let db_url = &env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool: Pool<Postgres> = PgPool::connect(db_url.as_str()).await.expect("Failed to connect to Postgres");
    return pool;
}