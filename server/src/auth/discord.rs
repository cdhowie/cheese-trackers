//! Discord authentication.

use base64::prelude::*;
use chacha20poly1305::{
    AeadCore, XChaCha20Poly1305,
    aead::{Aead, Nonce, OsRng, Payload},
};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenUrl,
    basic::{BasicClient, BasicTokenResponse},
    reqwest::async_http_client,
};
use url::Url;

/// Discord authentication client.
///
/// This wraps an OAuth2 client as well as the cipher used to encrypt and
/// decrypt continuation tokens.
pub struct AuthClient {
    /// The OAuth2 client.
    client: BasicClient,
    /// The cipher used to encrypt and decrypt continuation tokens.
    continuation_token_cipher: XChaCha20Poly1305,
}

impl AuthClient {
    /// Creates a new client.
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

    /// Begins a new authentication attempt.
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

    /// Completes an authentication attempt.
    pub async fn complete(
        &self,
        code: String,
        csrf_token: &str,
        continuation_token: &str,
    ) -> Result<BasicTokenResponse, CompleteAuthenticationError<impl std::error::Error + use<>>>
    {
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

/// State of an in-progress authentication attempt.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct AuthState {
    /// The URL the user should visit to continue the authentication process.
    pub auth_url: Url,
    /// An encrypted continuation token, which must be supplied when completing
    /// the authentication attempt.
    pub continuation_token: String,
}

/// Errors that may occur while beginning a new authentication attempt.
#[derive(Debug, thiserror::Error)]
pub enum AuthStateCreateError {
    /// The continuation token could not be encrypted.
    #[error("token encryption failed: {0}")]
    TokenEncrypt(chacha20poly1305::Error),
}

/// Creates a continuation token.
///
/// `cipher` is the encryption cipher to use, `verifier_secret` is the OAuth2
/// verifier secret, and `csrf_token` is the CSRF token for the authentication
/// attempt.
///
/// The verifier secret is encrypted using the provided cipher, using the CSRF
/// token as additional associated data.  This technique omits the CSRF token
/// from the encrypted payload, but requires exactly the same CSRF token be
/// provided to successfully decrypt the token.  Since the user is required to
/// provide the CSRF token to complete authentication, this allows us to verify
/// the validity of the token without having to store it anywhere.
///
/// The encrypted token is base64-encoded before being returned.
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

/// Errors that may occur when decryption a continuation token.
#[derive(Debug, thiserror::Error)]
pub enum ContinuationTokenDecryptError {
    /// The token is not valid base64.
    #[error("failed to base64-decode token: {0}")]
    Base64Decode(base64::DecodeError),
    /// The token's header is invalid.
    #[error("invalid token header")]
    InvalidHeader,
    /// Decryption of the token failed.
    #[error("token decryption failed: {0}")]
    Decrypt(chacha20poly1305::Error),
    /// The verifier secret within the encrypted token contains an invalid UTF-8
    /// sequence.
    #[error("invalid verifier secret UTF-8")]
    InvalidVerifierSecretUtf8,
}

/// Decrypts a continuation token.
///
/// `token` is the base64-encoded token, `csrf_token` is the CSRF token for the
/// request, and `cipher` is the encryption cipher.
///
/// If a different CSRF token is provided than was used to encrypt the token,
/// decryption will fail.
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

/// Errors that may occur when completing an authentication attempt.
#[derive(Debug, thiserror::Error)]
pub enum CompleteAuthenticationError<RTE> {
    /// The provided continuation token could not be decrypted.
    #[error("failed to decrypt continuation token: {0}")]
    ContinuationTokenDecrypt(ContinuationTokenDecryptError),
    /// The OAuth2 code exchange operation failed.
    #[error("failed to request token from oauth2 endpoint: {0}")]
    RequestToken(RTE),
}
