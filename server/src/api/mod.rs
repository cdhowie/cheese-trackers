//! API endpoints and related facilities.

use std::{future::ready, sync::Arc};

use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware,
    response::IntoResponse,
};
use futures::TryStreamExt;

use crate::{
    conf::Banner,
    db::{DataAccess, DataAccessProvider, model::JsErrorInsertion},
    logging::UnexpectedResultExt,
    send_hack::send_stream,
    state::AppState,
};

pub mod auth;
pub mod dashboard;
pub mod tracker;
pub mod user;

/// Creates a new API router with the provided state.
pub fn create_router<D>(state: Arc<AppState<D>>) -> axum::Router<()>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    use axum::routing::*;

    axum::Router::new()
        .route("/auth/begin", get(auth::begin_discord_auth))
        .route("/auth/complete", post(auth::complete_discord_auth))
        .route("/dashboard/tracker", get(dashboard::get_dashboard_trackers))
        .route("/tracker", post(tracker::create_tracker))
        .route("/tracker/{tracker_id}", get(tracker::get_tracker))
        .route("/tracker/{tracker_id}", put(tracker::update_tracker))
        .route(
            "/tracker/{tracker_id}/game/{game_id}",
            put(tracker::update_game),
        )
        .route(
            "/tracker/{tracker_id}/hint/{hint_id}",
            put(tracker::update_hint),
        )
        .route(
            "/tracker/{tracker_id}/dashboard_override",
            get(tracker::get_tracker_dashboard_override),
        )
        .route(
            "/tracker/{tracker_id}/dashboard_override",
            put(tracker::put_tracker_dashboard_override),
        )
        .route("/user/self/api_key", get(user::get_api_key))
        .route("/user/self/api_key", post(user::reset_api_key))
        .route("/user/self/api_key", delete(user::clear_api_key))
        .route("/user/self/settings", get(user::get_settings))
        .route("/user/self/settings", put(user::put_settings))
        .route("/settings", get(get_settings))
        .route("/jserror", post(create_js_error))
        // Since UI settings are in a header added by middleware, this no-op
        // endpoint allows fetching the UI settings without having to make a
        // dummy request to another endpoint.
        .route("/ping", get(|| ready(StatusCode::NO_CONTENT)))
        // Add the x-ct-settings header.
        .layer(middleware::from_fn_with_state(
            state.clone(),
            |state: State<Arc<AppState<D>>>, req: Request, next: middleware::Next| async move {
                let mut res = next.run(req).await;
                res.headers_mut()
                    .insert("x-ct-settings", state.ui_settings_header.clone());
                res
            },
        ))
        .with_state(state)
        // Add the cache-control header.
        .layer(middleware::from_fn(
            |req: Request, next: middleware::Next| async move {
                let mut res = next.run(req).await;
                res.headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                res
            },
        ))
}

/// UI settings.
///
/// The API router will encode this as JSON and put it in the `x-ct-settings`
/// response header for every request.  This allows the frontend to detect when
/// a new version is available as well as update the displayed banners every
/// time a request is made.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UiSettings {
    /// The Git commit identifier for the current version.
    ///
    /// This will be `"dev"` in development environments.
    pub build_version: &'static str,
    /// Name of the entity providing this instance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hoster: Option<String>,
    /// Banners that should be displayed in the frontend.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub banners: Vec<Banner>,
}

/// `GET /api/settings`: Get the current [UI settings](UiSettings).
///
/// Deprecated; replaced with the `x-ct-settings` response header, which is
/// automatically added in middleware.  This endpoint should be removed after
/// enough time has passed for all users to refresh their local CT version.
async fn get_settings<D>(State(state): State<Arc<AppState<D>>>) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json")],
        state.ui_settings_header.as_bytes().to_owned(),
    )
}

/// Request body for [`create_js_error`].
#[derive(Debug, Clone, serde::Deserialize)]
struct CreateJsErrorRequest {
    /// The ID of the user that generated the error, if the user is authenticated.
    pub ct_user_id: Option<i32>,
    /// The error represented as text.
    pub error: String,
}

/// `POST /jserror`: Log a JavaScript error.
///
/// This endpoint allows unhandled errors in the frontend to be captured and
/// investigated later.
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

        send_stream(db.create_js_errors([JsErrorInsertion {
            ct_user_id: request.ct_user_id,
            error: request.error,
        }]))
        .try_for_each(|_| std::future::ready(Ok(())))
        .await
        .unexpected()
    });

    StatusCode::ACCEPTED
}
