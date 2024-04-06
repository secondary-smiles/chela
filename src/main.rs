use std::net::SocketAddr;

use axum::routing::{get, post};
use axum::Router;

use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

use info_utils::prelude::*;

pub mod get;
pub mod post;

const LISTEN_ADDRESS: &'static str = "0.0.0.0:3000";

#[derive(Clone)]
pub struct ServerState {
    pub db_pool: Pool<Postgres>,
    pub host: String,
}

#[derive(Debug, Clone)]
pub struct UrlRow {
    pub index: i32,
    pub id: String,
    pub url: String,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let db_pool = init_db().await?;

    let server_state = ServerState {
        db_pool,
        host: "trkt.in".to_string(),
    };

    let router = init_routes(server_state)?;
    let listener = tokio::net::TcpListener::bind(LISTEN_ADDRESS).await?;
    log!("Listening at {}", LISTEN_ADDRESS);
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

async fn init_db() -> eyre::Result<Pool<Postgres>> {
    let db_pool = PgPoolOptions::new()
        .max_connections(15)
        .connect(std::env::var("DATABASE_URL")?.as_str())
        .await?;
    log!("Successfully connected to database");

    sqlx::query!("CREATE SCHEMA IF NOT EXISTS chela")
        .execute(&db_pool)
        .await?;
    log!("Created schema chela");

    sqlx::query!(
        "
CREATE TABLE IF NOT EXISTS chela.urls (
    index SERIAL PRIMARY KEY,
    id TEXT NOT NULL UNIQUE,
    url TEXT NOT NULL
)
        ",
    )
    .execute(&db_pool)
    .await?;
    log!("Created table chela.urls");

    sqlx::query!(
        "
CREATE TABLE IF NOT EXISTS chela.tracking (
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    id TEXT NOT NULL,
    ip TEXT NOT NULL,
    referrer TEXT,
    user_agent TEXT
)
        ",
    )
    .execute(&db_pool)
    .await?;
    log!("Created table chela.tracking");

    Ok(db_pool)
}

fn init_routes(state: ServerState) -> eyre::Result<Router> {
    let router = Router::new()
        .route("/", get(get::get_index))
        .route("/:id", get(get::get_id))
        .route("/", post(post::create_link))
        .layer(axum::Extension(state));

    Ok(router)
}
