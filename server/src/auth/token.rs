//! Authentication tokens.

use std::{borrow::Cow, sync::Arc};

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, StatusCode},
};
use futures::{future::BoxFuture, FutureExt};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::{
    db::{model::CtUser, DataAccess, DataAccessProvider},
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

/// Extracts a [`CtUser`] from a request, authenticated by a bearer token.
///
/// Extraction will fail if a token was not provided, the token is invalid, the
/// user ID encoded in the token is not present in the database, or a database
/// error occurs.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser(pub CtUser);

impl<D> FromRequestParts<Arc<AppState<D>>> for AuthenticatedUser
where
    D: DataAccessProvider + Send + Sync,
{
    type Rejection = StatusCode;

    fn from_request_parts<'p, 's, 'f>(
        parts: &'p mut axum::http::request::Parts,
        state: &'s Arc<AppState<D>>,
    ) -> BoxFuture<'f, Result<Self, Self::Rejection>>
    where
        'p: 'f,
        's: 'f,
        Self: 'f,
    {
        async move {
            let token = parts
                .headers
                .get(AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .and_then(|v| state.token_processor.decode(v).ok())
                .ok_or(StatusCode::UNAUTHORIZED)?;

            let user = state
                .data_provider
                .create_data_access()
                .await
                .unexpected()?
                .get_ct_user_by_id(token.sub)
                .await
                .unexpected()?
                .ok_or(StatusCode::UNAUTHORIZED)?;

            Ok(Self(user))
        }
        .boxed()
    }
}
