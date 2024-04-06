use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Path};
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use axum::Extension;

use info_utils::prelude::*;

use crate::ServerState;
use crate::UrlRow;

pub async fn get_index() -> Html<&'static str> {
    Html("hello, world!")
}

pub async fn get_id(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Extension(state): Extension<ServerState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut show_request = false;
    log!("Request for '{}' from {}", id.clone(), addr.ip());
    let mut use_id = id;
    if use_id.ends_with('+') {
        show_request = true;
        use_id.pop();
    }

    let item = sqlx::query_as!(UrlRow, "SELECT * FROM chela.urls WHERE id = $1", use_id)
        .fetch_one(&state.db_pool)
        .await;
    if let Ok(it) = item {
        if url::Url::parse(&it.url).is_ok() {
            if show_request {
                return Html(format!(
                    "<pre>http://{}/{} -> <a href={}>{}</a></pre>",
                    state.host, it.id, it.url, it.url
                ))
                .into_response();
            } else {
                log!("Redirecting {} -> {}", it.id, it.url);
                save_analytics(headers, it.clone(), addr, state).await;
                return Redirect::temporary(it.url.as_str()).into_response();
            }
        }
    }

    return (StatusCode::NOT_FOUND, Html("<pre>404</pre>")).into_response();
}

pub async fn save_analytics(
    headers: HeaderMap,
    item: UrlRow,
    addr: SocketAddr,
    state: ServerState,
) {
    let id = item.id;
    let ip = addr.ip().to_string();
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

    let res = sqlx::query!(
        "
INSERT INTO chela.tracking (id,ip,referrer,user_agent) 
VALUES ($1,$2,$3,$4)
       ",
        id,
        ip,
        referer,
        user_agent
    )
    .execute(&state.db_pool)
    .await;

    if res.is_ok() {
        log!("Saved analytics for '{id}' from {ip}");
    }
}
