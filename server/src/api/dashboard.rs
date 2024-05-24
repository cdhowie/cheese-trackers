use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use futures::TryStreamExt;

use crate::{
    auth::token::AuthenticatedUser,
    db::{DataAccess, DataAccessProvider},
    logging::UnexpectedResultExt,
    state::AppState,
};

pub async fn get_dashboard_trackers<D>(
    State(state): State<Arc<AppState<D>>>,
    user: AuthenticatedUser,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    Ok(Json(
        db.get_dashboard_trackers(user.0.id)
            .try_collect::<Vec<_>>()
            .await
            .unexpected()?,
    ))
}
