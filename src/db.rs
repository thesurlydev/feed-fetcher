use std::env;
use sqlx::{PgPool, Pool, Postgres, query_as, Row};
use crate::SourceType;

pub(crate) async fn source_types() -> Result<Vec<SourceType>, sqlx::Error> {
    let pool: Pool<Postgres> = PgPool::connect(&env::var("DATABASE_URL").unwrap()).await?;

    query_as!(SourceType, r#"SELECT * FROM source_type"#)
        .fetch_all(&pool)
        .await
}

