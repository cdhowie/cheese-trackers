//! Dashboard endpoints.

use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use serde::Serialize;

use crate::{
    api::tracker::UrlEncodedTrackerId,
    auth::token::AuthenticatedUser,
    db::{DataAccess, DataAccessProvider, model::ApTrackerDashboard},
    logging::UnexpectedResultExt,
    state::AppState,
};

/// `GET /dashboard/tracker`: Get trackers to display on the dashboard.
pub async fn get_dashboard_trackers<D>(
    State(state): State<Arc<AppState<D>>>,
    user: AuthenticatedUser,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    // Based on ApTrackerDashboard.  We need our own type because we have to
    // serialize tracker_id differently.
    #[derive(Debug, Clone, Serialize)]
    pub struct DashboardTracker {
        pub id: i32,
        pub tracker_id: UrlEncodedTrackerId,
        pub title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub owner_ct_user_id: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub owner_discord_username: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub last_activity: Option<DateTime<Utc>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub dashboard_override_visibility: Option<bool>,
        pub room_link: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub last_port: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub last_port_is_stale: Option<bool>,
    }

    impl From<ApTrackerDashboard> for DashboardTracker {
        fn from(value: ApTrackerDashboard) -> Self {
            Self {
                id: value.id,
                tracker_id: value.tracker_id.into(),
                title: value.title,
                owner_ct_user_id: value.owner_ct_user_id,
                owner_discord_username: value.owner_discord_username,
                last_activity: value.last_activity,
                dashboard_override_visibility: value.dashboard_override_visibility,
                room_link: value.room_link,
                last_port: value.last_port,
                // TODO: This check is not completely accurate; it will falsely
                // report a port as not stale if the port was checked recently,
                // but the room is not active.
                last_port_is_stale: value.next_port_check_at.map(|d| d < Utc::now()),
            }
        }
    }

    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    Ok(Json(
        db.get_dashboard_trackers(user.user.id)
            .map_ok(Into::into)
            .try_collect::<Vec<DashboardTracker>>()
            .await
            .unexpected()?,
    ))
}
