use std::net::SocketAddr;

use url::Url;

use axum::routing::{get, post};
use axum::Router;

use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

use sqids::Sqids;

use serde::Deserialize;

use info_utils::prelude::*;

pub mod get;
pub mod post;

#[derive(Clone)]
pub struct ServerState {
    pub db_pool: Pool<Postgres>,
    pub host: String,
    pub sqids: Sqids,
    pub main_page_redirect: Option<Url>,
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq, Eq)]
pub struct UrlRow {
    pub index: i64,
    pub id: String,
    pub url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CreateForm {
    pub id: String,
    pub url: url::Url,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let db_pool = init_db().await?;
    let host = std::env::var("CHELA_HOST").unwrap_or("localhost".to_string());
    let sqids = Sqids::builder()
        .alphabet(
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
                .chars()
                .collect(),
        )
        .blocklist(["create".to_string()].into())
        .build()?;
    let main_page_redirect = std::env::var("CHELA_MAIN_PAGE_REDIRECT").unwrap_or_default();
    let server_state = ServerState {
        db_pool,
        host,
        sqids,
        main_page_redirect: Url::parse(&main_page_redirect).ok(),
    };

    let address = std::env::var("CHELA_LISTEN_ADDRESS").unwrap_or("0.0.0.0".to_string());
    let port = 3000;

    let router = init_routes(server_state);
    let listener = tokio::net::TcpListener::bind(format!("{address}:{port}")).await?;
    log!("Listening at {}:{}", address, port);
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
        .connect(
            std::env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set")
                .as_str(),
        )
        .await?;
    log!("Successfully connected to database");

    sqlx::query("CREATE SCHEMA IF NOT EXISTS chela")
        .execute(&db_pool)
        .await?;
    log!("Created schema chela");

    sqlx::query(
        "
CREATE TABLE IF NOT EXISTS chela.urls (
    index BIGSERIAL PRIMARY KEY,
    id TEXT NOT NULL UNIQUE,
    url TEXT NOT NULL
)
        ",
    )
    .execute(&db_pool)
    .await?;
    log!("Created table chela.urls");

    sqlx::query(
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

fn init_routes(state: ServerState) -> Router {
    Router::new()
        .route("/", get(get::index))
        .route("/:id", get(get::id))
        .route("/create", get(get::create_id))
        .route("/", post(post::create_link))
        .layer(axum::Extension(state))
}
