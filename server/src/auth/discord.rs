use base64::prelude::*;
use chacha20poly1305::{
    aead::{Aead, Nonce, OsRng, Payload},
    AeadCore, XChaCha20Poly1305,
};
use oauth2::{
    basic::{BasicClient, BasicTokenResponse},
    reqwest::async_http_client,
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenUrl,
};
use url::Url;

pub struct AuthClient {
    client: BasicClient,
    continuation_token_cipher: XChaCha20Poly1305,
}

impl AuthClient {
    pub fn new(
        client_id: String,
        client_secret: String,
        public_url: &Url,
        continuation_token_cipher: XChaCha20Poly1305,
    ) -> Self {
        let client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new("https://discord.com/oauth2/authorize".to_owned()).unwrap(),
            Some(TokenUrl::new("https://discord.com/api/oauth2/token".to_owned()).unwrap()),
        )
        .set_redirect_uri(RedirectUrl::from_url(
            public_url.join("auth/complete").unwrap(),
        ));

        Self {
            client,
            continuation_token_cipher,
        }
    }

    pub fn begin(&self) -> Result<AuthState, AuthStateCreateError> {
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("identify".to_owned()))
            .set_pkce_challenge(challenge)
            .url();

        let continuation_token = encrypt_continuation_token(
            &self.continuation_token_cipher,
            verifier.secret(),
            csrf_token.secret(),
        )
        .map_err(AuthStateCreateError::TokenEncrypt)?;

        Ok(AuthState {
            auth_url,
            continuation_token,
        })
    }

    pub async fn complete(
        &self,
        code: String,
        csrf_token: &str,
        continuation_token: &str,
    ) -> Result<BasicTokenResponse, CompleteAuthenticationError<impl std::error::Error>> {
        let verifier = decrypt_continuation_token(
            continuation_token,
            csrf_token,
            &self.continuation_token_cipher,
        )
        .map_err(CompleteAuthenticationError::ContinuationTokenDecrypt)?;

        self.client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(verifier)
            .request_async(async_http_client)
            .await
            .map_err(CompleteAuthenticationError::RequestToken)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AuthState {
    pub auth_url: Url,
    pub continuation_token: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthStateCreateError {
    #[error("token encryption failed: {0}")]
    TokenEncrypt(chacha20poly1305::Error),
}

fn encrypt_continuation_token(
    cipher: &XChaCha20Poly1305,
    verifier_secret: &str,
    csrf_token: &str,
) -> Result<String, chacha20poly1305::Error> {
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);

    let claims = cipher.encrypt(
        &nonce,
        Payload {
            msg: verifier_secret.as_bytes(),
            aad: csrf_token.as_bytes(),
        },
    )?;

    let mut data = Vec::with_capacity(2 + nonce.len() + claims.len());
    data.extend(b"1|");
    data.extend(nonce);
    data.extend(claims);

    Ok(BASE64_STANDARD.encode(&data))
}

#[derive(Debug, thiserror::Error)]
pub enum ContinuationTokenDecryptError {
    #[error("failed to base64-decode token: {0}")]
    Base64Decode(base64::DecodeError),
    #[error("invalid token header")]
    InvalidHeader,
    #[error("token decryption failed: {0}")]
    Decrypt(chacha20poly1305::Error),
    #[error("invalid verifier secret UTF-8")]
    InvalidVerifierSecretUtf8,
}

fn decrypt_continuation_token(
    token: &str,
    csrf_token: &str,
    cipher: &XChaCha20Poly1305,
) -> Result<PkceCodeVerifier, ContinuationTokenDecryptError> {
    let token = BASE64_STANDARD
        .decode(token)
        .map_err(ContinuationTokenDecryptError::Base64Decode)?;

    let next = token
        .strip_prefix(b"1|")
        .ok_or(ContinuationTokenDecryptError::InvalidHeader)?;

    let (nonce, message) = next.split_at(24);

    let verifier_secret = cipher
        .decrypt(
            Nonce::<XChaCha20Poly1305>::from_slice(nonce),
            Payload {
                msg: message,
                aad: csrf_token.as_bytes(),
            },
        )
        .map_err(ContinuationTokenDecryptError::Decrypt)?;

    Ok(PkceCodeVerifier::new(
        String::from_utf8(verifier_secret)
            .map_err(|_| ContinuationTokenDecryptError::InvalidVerifierSecretUtf8)?,
    ))
}

#[derive(Debug, thiserror::Error)]
pub enum CompleteAuthenticationError<RTE> {
    #[error("failed to decrypt continuation token: {0}")]
    ContinuationTokenDecrypt(ContinuationTokenDecryptError),
    #[error("failed to request token from oauth2 endpoint: {0}")]
    RequestToken(RTE),
}
