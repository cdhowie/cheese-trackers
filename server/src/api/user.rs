use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use uuid::Uuid;

use crate::{
    auth::token::TokenAuthenticatedUser,
    db::{model::CtUserIden, DataAccess, DataAccessProvider},
    logging::UnexpectedResultExt,
    state::AppState,
};

/// `GET /user/self/api_key`: Get current API key.
pub async fn get_api_key(
    TokenAuthenticatedUser(user): TokenAuthenticatedUser,
) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(user.api_key))
}

/// `POST /user/self/api_key`: Create new API key.
pub async fn reset_api_key<D>(
    State(state): State<Arc<AppState<D>>>,
    TokenAuthenticatedUser(mut user): TokenAuthenticatedUser,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    let new_key = Uuid::new_v4();

    user.api_key = Some(new_key);

    db.update_ct_user(user, &[CtUserIden::ApiKey])
        .await
        .unexpected()?;

    Ok(Json(new_key))
}

/// `DELETE /user/self/api_key`: Delete API key.
pub async fn clear_api_key<D>(
    State(state): State<Arc<AppState<D>>>,
    TokenAuthenticatedUser(mut user): TokenAuthenticatedUser,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    user.api_key = None;

    db.update_ct_user(user, &[CtUserIden::ApiKey])
        .await
        .unexpected()?;

    Ok(StatusCode::NO_CONTENT)
}
