use std::borrow::Cow;

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
    state::{GetDataAccessProvider, GetTokenProcessor},
};

pub type Result<T, E = jsonwebtoken::errors::Error> = std::result::Result<T, E>;

pub struct TokenProcessor {
    header: Header,
    encoding_key: EncodingKey,
    validity_duration_sec: u64,
    issuer: String,

    validation: Validation,
    decoding_key: DecodingKey,
}

impl TokenProcessor {
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

    pub fn decode(&self, token: &str) -> Result<TokenPayload<'static>> {
        jsonwebtoken::decode(token, &self.decoding_key, &self.validation).map(|d| d.claims)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenPayload<'a> {
    pub sub: i32,
    pub iat: u64,
    pub exp: u64,
    pub iss: Cow<'a, str>,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser(pub CtUser);

impl<S: GetTokenProcessor + GetDataAccessProvider + Sync> FromRequestParts<S>
    for AuthenticatedUser
{
    type Rejection = StatusCode;

    fn from_request_parts<'p, 's, 'f>(
        parts: &'p mut axum::http::request::Parts,
        state: &'s S,
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
                .and_then(|v| state.get_token_processor().decode(v).ok())
                .ok_or(StatusCode::UNAUTHORIZED)?;

            let user = state
                .get_data_provider()
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
