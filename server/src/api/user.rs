use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::token::{AuthenticatedUser, TokenAuthenticatedUser},
    db::{
        DataAccess, DataAccessProvider,
        model::{CtUser, CtUserIden},
    },
    logging::{UnexpectedResultExt, log},
    state::AppState,
};

/// `GET /user/self`: Get current user details.
pub async fn get_self(auth_user: AuthenticatedUser) -> impl IntoResponse {
    #[derive(Serialize)]
    pub struct UserProfile {
        pub id: i32,
        pub discord_username: String,
    }

    Json(UserProfile {
        id: auth_user.user.id,
        discord_username: auth_user.user.discord_username,
    })
}

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

#[derive(Deserialize, Serialize, Debug)]
pub struct UserSettings {
    pub is_away: bool,
}

impl UserSettings {
    pub const COLUMNS: [CtUserIden; 1] = [CtUserIden::IsAway];

    pub fn apply_to(self, user: &mut CtUser) {
        user.is_away = self.is_away;
    }
}

impl From<CtUser> for UserSettings {
    fn from(value: CtUser) -> Self {
        Self {
            is_away: value.is_away,
        }
    }
}

/// `GET /user/self/settings`: Get user settings.
pub async fn get_settings(user: AuthenticatedUser) -> impl IntoResponse {
    Json(UserSettings::from(user.user))
}

/// `PUT /user/self/settings`: Update user settings.
pub async fn put_settings<D>(
    State(state): State<Arc<AppState<D>>>,
    user: AuthenticatedUser,
    Json(settings): Json<UserSettings>,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    let mut user = user.user;

    settings.apply_to(&mut user);

    let user = db
        .update_ct_user(user, &UserSettings::COLUMNS)
        .await
        .unexpected()?
        .ok_or_else(|| {
            log!("User was deleted during put_settings");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(UserSettings::from(user)))
}
