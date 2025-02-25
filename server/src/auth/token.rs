//! Authentication tokens.

use std::{borrow::Cow, convert::Infallible, sync::Arc};

use axum::{
    extract::{FromRequestParts, OptionalFromRequestParts},
    http::{StatusCode, header::AUTHORIZATION},
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::{
    db::{DataAccess, DataAccessProvider, model::CtUser},
    logging::UnexpectedResultExt,
    state::AppState,
};

/// Type alias for results of JWT operations.
pub type Result<T, E = jsonwebtoken::errors::Error> = std::result::Result<T, E>;

/// Encodes and decodes authentication tokens.
pub struct TokenProcessor {
    /// Token header.
    header: Header,
    /// Token encryption key.
    encoding_key: EncodingKey,
    /// Duration in seconds for which generated tokens should be valid.
    validity_duration_sec: u64,
    /// Token issuer, placed in the `iss` payload field.
    issuer: String,

    /// Cached validation options.
    validation: Validation,
    /// Token decryption key.
    decoding_key: DecodingKey,
}

impl TokenProcessor {
    /// Creates a new token processor.
    pub fn new(
        header: Header,
        key: &str,
        issuer: String,
        validity_duration: chrono::Duration,
    ) -> Self {
        let mut validation = Validation::new(header.alg);
        validation.set_issuer(&[issuer.as_str()]);

        Self {
            header,
            encoding_key: EncodingKey::from_secret(key.as_bytes()),
            validity_duration_sec: u64::try_from(validity_duration.num_seconds())
                .expect("couldn't convert validity duration to u64"),
            issuer,

            validation,
            decoding_key: DecodingKey::from_secret(key.as_bytes()),
        }
    }

    /// Issues a new token for the given user ID.
    pub fn encode(&self, user_id: i32) -> Result<String> {
        let now = jsonwebtoken::get_current_timestamp();

        let payload = TokenPayload {
            sub: user_id,
            iat: now,
            exp: now + self.validity_duration_sec,
            iss: self.issuer.as_str().into(),
        };

        jsonwebtoken::encode(&self.header, &payload, &self.encoding_key)
    }

    /// Decodes a token.
    pub fn decode(&self, token: &str) -> Result<TokenPayload<'static>> {
        jsonwebtoken::decode(token, &self.decoding_key, &self.validation).map(|d| d.claims)
    }
}

/// Authentication token claims.
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenPayload<'a> {
    /// Local user ID.
    pub sub: i32,
    /// JWT timestamp the token was created.
    pub iat: u64,
    /// JWT timestamp the token expires at.
    pub exp: u64,
    /// Token issuer.
    pub iss: Cow<'a, str>,
}

/// Extracts a [`CtUser`] from a request, authenticated by a session token or an
/// API key.
///
/// Extraction will fail if a token or key was not provided, the token or key is
/// invalid, the user ID encoded in the token is not present in the database, or
/// a database error occurs.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user: CtUser,
    pub source: AuthenticationSource,
}

/// Identifies the source of a user's authentication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticationSource {
    SessionToken,
    ApiKey,
}

impl<D> FromRequestParts<Arc<AppState<D>>> for AuthenticatedUser
where
    D: DataAccessProvider + Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &Arc<AppState<D>>,
    ) -> Result<Self, Self::Rejection> {
        let bearer_token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(StatusCode::UNAUTHORIZED)?;

        match bearer_token.parse() {
            Ok(key) => {
                let user = state
                    .data_provider
                    .create_data_access()
                    .await
                    .unexpected()?
                    .get_ct_user_by_api_key(key)
                    .await
                    .ok()
                    .flatten()
                    .ok_or(StatusCode::UNAUTHORIZED)?;

                Ok(Self {
                    user,
                    source: AuthenticationSource::ApiKey,
                })
            }

            Err(_) => {
                let token = state
                    .token_processor
                    .decode(bearer_token)
                    .map_err(|_| StatusCode::UNAUTHORIZED)?;

                let user = state
                    .data_provider
                    .create_data_access()
                    .await
                    .unexpected()?
                    .get_ct_user_by_id(token.sub)
                    .await
                    .unexpected()?
                    .ok_or(StatusCode::UNAUTHORIZED)?;

                Ok(Self {
                    user,
                    source: AuthenticationSource::SessionToken,
                })
            }
        }
    }
}

impl<D> OptionalFromRequestParts<Arc<AppState<D>>> for AuthenticatedUser
where
    D: DataAccessProvider + Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &Arc<AppState<D>>,
    ) -> Result<Option<Self>, Self::Rejection> {
        match <Self as FromRequestParts<_>>::from_request_parts(parts, state).await {
            Ok(v) => Ok(Some(v)),
            Err(_) => Ok(None),
        }
    }
}

/// Extracts a [`CtUser`] from a request, authenticated by a session token.
///
/// Extraction will fail if a token was not provided, the token is invalid, the
/// user ID encoded in the token is not present in the database, or a database
/// error occurs.
///
/// Note this extractor specifically excludes API keys.
#[derive(Debug, Clone)]
pub struct TokenAuthenticatedUser(pub CtUser);

impl<S> FromRequestParts<S> for TokenAuthenticatedUser
where
    AuthenticatedUser: FromRequestParts<S, Rejection = StatusCode>,
    S: Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let user =
            <AuthenticatedUser as FromRequestParts<S>>::from_request_parts(parts, state).await?;

        match user.source {
            AuthenticationSource::SessionToken => Ok(Self(user.user)),
            AuthenticationSource::ApiKey => Err(StatusCode::FORBIDDEN),
        }
    }
}
