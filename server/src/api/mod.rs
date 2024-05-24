use std::{future::ready, sync::Arc};

use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware,
    response::IntoResponse,
    Json,
};
use futures::TryStreamExt;

use crate::{
    conf::Banner,
    db::{model::JsError, DataAccess, DataAccessProvider},
    logging::UnexpectedResultExt,
    state::AppState,
};

pub mod auth;
pub mod dashboard;
pub mod tracker;

pub fn create_router<D>(state: Arc<AppState<D>>) -> axum::Router<()>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    use axum::routing::*;

    axum::Router::new()
        .route("/auth/begin", get(auth::begin_discord_auth))
        .route("/auth/complete", post(auth::complete_discord_auth))
        .route("/dashboard/tracker", get(dashboard::get_dashboard_trackers))
        .route("/tracker/:tracker_id", get(tracker::get_tracker))
        .route("/tracker/:tracker_id", put(tracker::update_tracker))
        .route(
            "/tracker/:tracker_id/game/:game_id",
            put(tracker::update_game),
        )
        .route(
            "/tracker/:tracker_id/hint/:hint_id",
            put(tracker::update_hint),
        )
        .route("/settings", get(get_settings))
        .route("/jserror", post(create_js_error))
        // Since UI settings are in a header added by middleware, this no-op
        // endpoint allows fetching the UI settings without having to make a
        // dummy request to another endpoint.
        .route("/ping", get(|| ready(StatusCode::NO_CONTENT)))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            add_uisettings_header,
        ))
        .with_state(state)
        .layer(middleware::from_fn(
            |req: Request, next: middleware::Next| async move {
                let mut res = next.run(req).await;
                res.headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                res
            },
        ))
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UiSettings {
    pub build_version: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub banners: Vec<Banner>,
}

// Deprecated; replaced with x-ct-settings header added in middleware.  Remove
// this after enough time has passed for all users to refresh their local CT
// version.
async fn get_settings<D>(State(state): State<Arc<AppState<D>>>) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json")],
        state.ui_settings_header.as_bytes().to_owned(),
    )
}

async fn add_uisettings_header<D>(
    state: State<Arc<AppState<D>>>,
    req: Request,
    next: middleware::Next,
) -> axum::response::Response {
    let mut res = next.run(req).await;
    res.headers_mut()
        .insert("x-ct-settings", state.ui_settings_header.clone());
    res
}

#[derive(Debug, Clone, serde::Deserialize)]
struct CreateJsErrorRequest {
    pub ct_user_id: Option<i32>,
    pub error: String,
}

async fn create_js_error<D>(
    State(state): State<Arc<AppState<D>>>,
    Json(request): Json<CreateJsErrorRequest>,
) -> StatusCode
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    // We don't need to inform the client if this fails, so perform the
    // insertion in the background and respond immediately.
    tokio::spawn(async move {
        let mut db = state
            .data_provider
            .create_data_access()
            .await
            .unexpected()?;

        db.create_js_errors([JsError {
            id: 0,
            ct_user_id: request.ct_user_id,
            error: request.error,
        }])
        .try_for_each(|_| std::future::ready(Ok(())))
        .await
        .unexpected()
    });

    StatusCode::ACCEPTED
}
