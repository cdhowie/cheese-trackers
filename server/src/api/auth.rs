//! Authentication endpoints.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use chrono::Utc;
use futures::TryStreamExt;
use oauth2::TokenResponse;

use crate::{
    db::{
        model::{CtUser, CtUserIden},
        DataAccess, DataAccessProvider, Transactable, Transaction,
    },
    logging::UnexpectedResultExt,
    state::AppState,
};

/// `GET /auth/begin`: Begin Discord authentication.
pub async fn begin_discord_auth<D>(
    State(state): State<Arc<AppState<D>>>,
) -> Result<impl IntoResponse, StatusCode> {
    state.auth_client.begin().unexpected().map(Json)
}

/// Request body for [`complete_discord_auth`].
#[derive(serde::Deserialize)]
pub struct CompleteAuthRequest {
    pub code: String,
    pub state: String,
    pub continuation_token: String,
}

/// `POST /auth/complete`: Complete Discord authentication.
pub async fn complete_discord_auth<D>(
    State(state): State<Arc<AppState<D>>>,
    Json(request): Json<CompleteAuthRequest>,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    #[derive(Debug, thiserror::Error)]
    #[error("missing refresh token")]
    struct MissingRefreshTokenError;

    #[derive(Debug, thiserror::Error)]
    #[error("failed to insert Discord user {0} but user doesn't exist")]
    struct MissingUserError(u64);

    #[derive(serde::Serialize)]
    struct Response {
        token: String,
        user_id: i32,
        discord_username: String,
    }

    let token = state
        .auth_client
        .complete(request.code, &request.state, &request.continuation_token)
        .await
        .unexpected()?;

    let expires_at = Utc::now()
        + token
            .expires_in()
            .and_then(|d| chrono::Duration::from_std(d).ok())
            .unwrap_or_else(|| chrono::Duration::days(1));

    let user_info = serenity::all::User::from(
        serenity::http::Http::new(&format!("Bearer {}", token.access_token().secret()))
            .get_current_user()
            .await
            .unexpected()?,
    );

    // This may overflow, which is fine.  PostgreSQL doesn't support unsigned
    // types; the alternative is NUMERIC or TEXT.
    //
    // The domains of i64 and u64 are the same size, and the cast is reversible,
    // so we can cast back to u64 later to retrieve the true user ID.
    let discord_user_id = user_info.id.get() as i64;

    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    let mut tx = db.begin().await.unexpected()?;

    // Try to insert the user.  If the user already exists, we'll fetch it
    // below.
    let r = tx
        .create_ct_users([CtUser {
            id: 0,
            discord_access_token: token.access_token().secret().to_owned(),
            discord_access_token_expires_at: expires_at,
            discord_refresh_token: token
                .refresh_token()
                .ok_or(MissingRefreshTokenError)
                .unexpected()?
                .secret()
                .to_owned(),
            discord_user_id,
            discord_username: user_info.name.clone(),
        }])
        .try_next()
        .await;

    let ct_user = match r {
        Err(e)
            if e.as_database_error()
                .is_some_and(|dbe| dbe.is_unique_violation()) =>
        {
            // Restart failed transaction.
            tx.rollback().await.unexpected()?;
            tx = db.begin().await.unexpected()?;

            Ok(None)
        }
        v => v,
    }
    .unexpected()?;

    let ct_user = match ct_user {
        Some(u) => u,
        None => {
            let mut u = tx
                .get_ct_user_by_discord_user_id(discord_user_id)
                .await
                .unexpected()?
                .ok_or(MissingUserError(user_info.id.get()))
                .unexpected()?;

            // The user already existed.  Update their token and username.
            u.discord_access_token = token.access_token().secret().to_owned();
            u.discord_access_token_expires_at = expires_at;
            u.discord_refresh_token = token
                .refresh_token()
                .ok_or(MissingRefreshTokenError)
                .unexpected()?
                .secret()
                .to_owned();
            u.discord_username = user_info.name;

            tx.update_ct_user(
                u.clone(),
                &[
                    CtUserIden::DiscordAccessToken,
                    CtUserIden::DiscordAccessTokenExpiresAt,
                    CtUserIden::DiscordRefreshToken,
                    CtUserIden::DiscordUsername,
                ],
            )
            .await
            .unexpected()?;

            u
        }
    };

    tx.commit().await.unexpected()?;

    Ok(Json(Response {
        token: state.token_processor.encode(ct_user.id).unexpected()?,
        user_id: ct_user.id,
        discord_username: ct_user.discord_username,
    }))
}
