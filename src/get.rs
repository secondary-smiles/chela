use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Path};
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use axum::Extension;

use info_utils::prelude::*;

use crate::ServerState;
use crate::UdsConnectInfo;
use crate::UrlRow;

pub async fn index(Extension(state): Extension<ServerState>) -> impl IntoResponse {
    if let Some(redirect) = state.main_page_redirect {
        return Redirect::temporary(redirect.as_str()).into_response();
    }

    Html(format!(
        r#"
    <!DOCTYPE html>
    <html>
        <head>
            <title>{} URL Shortener</title>
        </head>
    </html>
    <body>
        <pre>{} URL shortener</pre>
        <a href="/create">create</a>
    </body>
         "#,
        state.host, state.host
    ))
    .into_response()
}

pub async fn id_unix(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<UdsConnectInfo>,
    Extension(state): Extension<ServerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let ip = format!("{:?}", addr.peer_addr);
    run_id(headers, ip, state, id).await
}

/// # Panics
/// Will panic if `parse()` fails
pub async fn id(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Extension(state): Extension<ServerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let ip = get_ip(&headers, addr, &state).unwrap_or_default();
    run_id(headers, ip, state, id).await
}

async fn run_id(
    headers: HeaderMap,
    ip: String,
    state: ServerState,
    id: String,
) -> impl IntoResponse {
    let mut show_request = false;
    log!("Request for '{}' from {}", id.clone(), ip);
    let mut use_id = id;
    if use_id.ends_with('+') {
        show_request = true;
        use_id.pop();
    }

    let item: Result<UrlRow, sqlx::Error> =
        sqlx::query_as("SELECT * FROM chela.urls WHERE id = $1")
            .bind(use_id.clone())
            .fetch_one(&state.db_pool)
            .await;
    if let Ok(it) = item {
        if url::Url::parse(&it.url).is_ok() {
            if show_request {
                return Html(format!(
                    r#"<pre>http://{}/{} -> <a href="{}"">{}</a></pre>"#,
                    state.host, it.id, it.url, it.url
                ))
                .into_response();
            }
            log!("Redirecting {} -> {}", it.id, it.url);
            save_analytics(headers, it.clone(), ip, state).await;
            let mut response_headers = HeaderMap::new();
            response_headers.insert("Cache-Control", "private, max-age=90".parse().unwrap());
            response_headers.insert("Location", it.url.parse().unwrap());
            return (
                StatusCode::MOVED_PERMANENTLY,
                response_headers,
                Html(format!(
                    r#"Redirecting to <a href="{}">{}</a>"#,
                    it.url, it.url
                )),
            )
                .into_response();
        }
    } else {
        warn!("'{}' not found.", use_id);
        return (
            StatusCode::NOT_FOUND,
            Html("<pre>Not found.</pre>".to_string()),
        )
            .into_response();
    }

    (StatusCode::NOT_FOUND, Html("<pre>Not found.</pre>")).into_response()
}

async fn save_analytics(headers: HeaderMap, item: UrlRow, ip: String, state: ServerState) {
    let id = item.id;
    let referer = match headers.get("referer") {
        Some(it) => {
            if let Ok(i) = it.to_str() {
                Some(i)
            } else {
                None
            }
        }
        None => None,
    };
    let user_agent = match headers.get("user-agent") {
        Some(it) => {
            if let Ok(i) = it.to_str() {
                Some(i)
            } else {
                None
            }
        }
        None => None,
    };

    let res = sqlx::query(
        "
INSERT INTO chela.tracking (id,ip,referrer,user_agent) 
VALUES ($1,$2,$3,$4)
       ",
    )
    .bind(id.clone())
    .bind(ip.clone())
    .bind(referer)
    .bind(user_agent)
    .execute(&state.db_pool)
    .await;

    if res.is_ok() {
        log!("Saved analytics for '{id}' from {}", ip);
    }
}

fn get_ip(headers: &HeaderMap, addr: SocketAddr, state: &ServerState) -> Option<String> {
    if state.behind_proxy {
        match headers.get("x-real-ip") {
            Some(it) => {
                if let Ok(i) = it.to_str() {
                    Some(i.to_string())
                } else {
                    None
                }
            }
            None => None,
        }
    } else {
        Some(addr.ip().to_string())
    }
}

pub async fn create_id(Extension(state): Extension<ServerState>) -> Html<String> {
    Html(format!(
        r#"
        <!DOCTYPE html>
        <html>
            <head>
                <title>{} URL Shortener</title>
            </head>
            <body>
                <form action="/" method="post">
                    <label for="url">
                        URL to shorten:
                        <input type="url" name="url" required>
                    </label>
                    <br />
                    <label for="id">
                        ID (optional):
                        <input type="text" name="id">
                    </label>
                    <br />
                    <input type="submit" value="create">
                </form>
            </body>
        </html>
         "#,
        state.host
    ))
}
