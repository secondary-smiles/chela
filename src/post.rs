use axum::extract::Form;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::Extension;

use info_utils::prelude::*;

use crate::CreateForm;
use crate::ServerState;
use crate::UrlRow;

#[derive(Debug, Clone, sqlx::FromRow, PartialEq, Eq)]
struct NextId {
    id: String,
    index: Option<i64>,
    exists: bool,
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq, Eq)]
struct NextIndex {
    new_index: Option<i64>,
}

pub async fn create_link(
    Extension(state): Extension<ServerState>,
    Form(form): Form<CreateForm>,
) -> impl IntoResponse {
    log!("Request to create '{}' -> {}", form.id, form.url.as_str());

    let try_id = generate_id(form.clone(), state.clone()).await;
    if let Ok(id) = try_id {
        if id.exists {
            log!("Serving cached id {} -> {}", id.id, form.url.as_str());
            return Html(format!(
                r#"<pre>http://{}/{} -> <a href="{}"">{}</a></pre>"#,
                state.host,
                id.id,
                form.url.as_str(),
                form.url.as_str(),
            ))
            .into_response();
        }
        let res;
        if let Some(index) = id.index {
            res = sqlx::query(
                "
INSERT INTO chela.urls (index,id,url)
VALUES ($1,$2,$3)
              ",
            )
            .bind(index)
            .bind(id.id.clone())
            .bind(form.url.as_str())
            .execute(&state.db_pool)
            .await;
        } else {
            res = sqlx::query(
                "
INSERT INTO chela.urls (id,url)
VALUES ($1,$2)
              ",
            )
            .bind(id.id.clone())
            .bind(form.url.as_str())
            .execute(&state.db_pool)
            .await;
        }

        match res {
            Ok(_) => {
                log!("Created new id {} -> {}", id.id, form.url.as_str());
                return (
                    StatusCode::OK,
                    Html(format!(
                        r#"<pre>http://{}/{} -> <a href="{}"">{}</a></pre>"#,
                        state.host,
                        id.id,
                        form.url.as_str(),
                        form.url.as_str(),
                    )),
                )
                    .into_response();
            }
            Err(err) => {
                warn!("{}", err);
                return (StatusCode::INTERNAL_SERVER_ERROR, Html("Internal error."))
                    .into_response();
            }
        }
    } else if let Err(err) = try_id {
        warn!("{}", err);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("Internal error: {err}")),
        )
            .into_response();
    }

    (StatusCode::INTERNAL_SERVER_ERROR, Html("Internal error.")).into_response()
}

async fn generate_id(form: CreateForm, state: ServerState) -> eyre::Result<NextId> {
    if form.id.is_empty() {
        let existing_row: Result<UrlRow, sqlx::Error> =
            sqlx::query_as("SELECT * FROM chela.urls WHERE url = $1")
                .bind(form.url.as_str())
                .fetch_one(&state.db_pool)
                .await;
        if let Ok(row) = existing_row {
            return Ok(NextId {
                id: row.id,
                index: None,
                exists: true,
            });
        }

        let next_index: NextIndex = sqlx::query_as(
            "SELECT nextval(pg_get_serial_sequence('chela.urls', 'index')) as new_index",
        )
        .fetch_one(&state.db_pool)
        .await?;

        if let Some(index) = next_index.new_index {
            let new_id = state.sqids.encode(&[index.try_into()?])?;
            return Ok(NextId {
                id: new_id,
                index: Some(index),
                exists: false,
            });
        }
    } else {
        let existing_row: Result<UrlRow, sqlx::Error> =
            sqlx::query_as("SELECT * FROM chela.urls WHERE id = $1")
                .bind(form.id.clone())
                .fetch_one(&state.db_pool)
                .await;
        if let Ok(row) = existing_row {
            if row.url == form.url.as_str() {
                return Ok(NextId {
                    id: row.id,
                    index: None,
                    exists: true,
                });
            }
            return Err(eyre::eyre!("id '{}' is already taken", row.id));
        }
        return Ok(NextId {
            id: form.id,
            index: None,
            exists: false,
        });
    }

    Err(eyre::eyre!("Internal error"))
}
