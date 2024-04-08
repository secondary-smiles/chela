use axum::extract::connect_info;
use axum::http::Request;
use axum::routing::{get, post};
use axum::Router;

use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server;

use info_utils::prelude::*;
use serde::Deserialize;
use sqids::Sqids;
use tower::Service;
use url::Url;

use std::env;
use std::sync::Arc;

pub mod get;
pub mod post;

#[derive(Clone)]
pub struct ServerState {
    pub db_pool: Pool<Postgres>,
    pub host: String,
    pub sqids: Sqids,
    pub main_page_redirect: Option<Url>,
    pub behind_proxy: bool,
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

#[derive(Clone)]
#[allow(dead_code)]
pub struct UdsConnectInfo {
    pub peer_addr: Arc<tokio::net::unix::SocketAddr>,
    pub peer_cred: tokio::net::unix::UCred,
}

impl connect_info::Connected<&tokio::net::UnixStream> for UdsConnectInfo {
    fn connect_info(target: &tokio::net::UnixStream) -> Self {
        let peer_addr = target.peer_addr().unwrap();
        let peer_cred = target.peer_cred().unwrap();

        Self {
            peer_addr: Arc::new(peer_addr),
            peer_cred,
        }
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let db_pool = init_db().await?;
    let host = env::var("CHELA_HOST").unwrap_or("localhost".to_string());
    let alphabet = env::var("CHELA_ALPHABET")
        .unwrap_or("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ".to_string());
    let sqids = Sqids::builder()
        .alphabet(alphabet.chars().collect())
        .blocklist(["create".to_string()].into())
        .build()?;
    let main_page_redirect = env::var("CHELA_MAIN_PAGE_REDIRECT").unwrap_or_default();
    let behind_proxy = env::var("CHELA_BEHIND_PROXY").is_ok();
    let server_state = ServerState {
        db_pool,
        host,
        sqids,
        main_page_redirect: Url::parse(&main_page_redirect).ok(),
        behind_proxy,
    };

    serve(server_state).await?;
    Ok(())
}

async fn serve(state: ServerState) -> eyre::Result<()> {
    let unix_socket = env::var("CHELA_UNIX_SOCKET").unwrap_or_default();
    if unix_socket.is_empty() {
        let router = Router::new()
            .route("/", get(get::index))
            .route("/:id", get(get::id))
            .route("/create", get(get::create_id))
            .route("/", post(post::create_link))
            .layer(axum::Extension(state));
        let address = env::var("CHELA_LISTEN_ADDRESS").unwrap_or("0.0.0.0".to_string());
        let port = 3000;
        let listener = tokio::net::TcpListener::bind(format!("{address}:{port}")).await?;
        log!("Listening at {}:{}", address, port);
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await?;
    } else {
        let router = Router::new()
            .route("/", get(get::index))
            .route("/:id", get(get::id_unix))
            .route("/create", get(get::create_id))
            .route("/", post(post::create_link))
            .layer(axum::Extension(state));
        let unix_socket_path = std::path::Path::new(&unix_socket);
        if unix_socket_path.exists() {
            tokio::fs::remove_file(unix_socket_path).await?;
        }
        let listener = tokio::net::UnixListener::bind(unix_socket_path)?;
        log!("Listening via Unix socket at {}", unix_socket);
        tokio::spawn(async move {
            let mut service = router.into_make_service_with_connect_info::<UdsConnectInfo>();
            loop {
                let (socket, _remote_addr) = listener.accept().await.unwrap();
                let tower_service = match service.call(&socket).await {
                    Ok(value) => value,
                    Err(err) => match err {},
                };

                tokio::spawn(async move {
                    let socket = TokioIo::new(socket);
                    let hyper_service =
                        hyper::service::service_fn(move |request: Request<Incoming>| {
                            tower_service.clone().call(request)
                        });

                    if let Err(err) = server::conn::auto::Builder::new(TokioExecutor::new())
                        .serve_connection_with_upgrades(socket, hyper_service)
                        .await
                    {
                        warn!("Failed to serve connection: {}", err);
                    }
                });
            }
        })
        .await?;
    }

    Ok(())
}

async fn init_db() -> eyre::Result<Pool<Postgres>> {
    let db_pool = PgPoolOptions::new()
        .max_connections(15)
        .connect(
            env::var("DATABASE_URL")
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
    url TEXT NOT NULL,
    custom_id BOOLEAN NOT NULL
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
    ip TEXT,
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
        .route("/create", get(get::create_id))
        .route("/", post(post::create_link))
        .layer(axum::Extension(state))
}
