//! Sign in with ChatGPT (official OAuth 2.0 + PKCE, loopback redirect on 127.0.0.1).
//!
//! Implements the flow documented at
//! https://developers.openai.com/siwc/token-sharing-open-source/sign-in
//! - First sign-in uses `client_id=dynamic_agent_client`; the callback returns an
//!   issued `client_id`, which we persist and reuse for later sign-ins/refresh.
//! - No client secret and no API key.
//! - Tokens live only in the OS credential store.

use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub const AUTHORIZE_URL: &str = "https://auth.openai.com/api/accounts/authorize";
pub const TOKEN_URL: &str = "https://auth.openai.com/api/accounts/oauth/token";
pub const OIDC_CONFIG_URL: &str = "https://auth.openai.com/.well-known/openid-configuration";
pub const ISSUER: &str = "https://auth.openai.com";
pub const RESOURCE: &str = "https://api.openai.com/v1";
pub const DYNAMIC_CLIENT: &str = "dynamic_agent_client";
pub const SCOPES: &str = "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";
pub const PLAN_SCOPE: &str = "chatgpt.tokens.use.direct";
pub const APP_NAME: &str = "InputFlow";
const CALLBACK_PATH: &str = "/auth/callback";
const PREFERRED_PORT: u16 = 1455;

/// Non-secret registration data, persisted in the config dir.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Registration {
    /// Stable opaque per-install host id (`urn:uuid:…`), never derived from user data.
    pub ext_agent_host_id: String,
    /// Issued client id from the first successful registration.
    pub client_id: Option<String>,
}

/// Secret token set — stored only in the OS credential store.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenSet {
    pub client_id: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub expires_at: i64,
    pub scope: String,
    pub account: Account,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub expires_in: Option<i64>,
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenError {
    error: Option<String>,
    error_description: Option<String>,
}

pub fn random_urlsafe(bytes: usize) -> String {
    let mut b = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut b);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b)
}

pub fn pkce_challenge(verifier: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub struct AuthRequest {
    pub url: String,
    pub verifier: String,
    pub state: String,
    pub nonce: String,
    pub redirect_uri: String,
    pub client_id: String,
    pub first_registration: bool,
}

pub fn build_authorize(reg: &Registration, redirect_uri: &str, id_token_hint: Option<&str>) -> AuthRequest {
    let verifier = random_urlsafe(48);
    let state = random_urlsafe(24);
    let nonce = random_urlsafe(24);
    let (client_id, first) = match &reg.client_id {
        Some(c) => (c.clone(), false),
        None => (DYNAMIC_CLIENT.to_string(), true),
    };
    let mut q: Vec<(&str, String)> = vec![
        ("client_id", client_id.clone()),
        ("response_type", "code".into()),
        ("redirect_uri", redirect_uri.into()),
        ("scope", SCOPES.into()),
        ("resource", RESOURCE.into()),
        ("state", state.clone()),
        ("nonce", nonce.clone()),
        ("code_challenge", pkce_challenge(&verifier)),
        ("code_challenge_method", "S256".into()),
        ("ext_agent_host_id", reg.ext_agent_host_id.clone()),
    ];
    if first {
        q.push(("agent_name_hint", APP_NAME.into()));
    } else if let Some(h) = id_token_hint {
        q.push(("id_token_hint", h.into()));
    }
    let qs = q.iter().map(|(k, v)| format!("{k}={}", urlencoding::encode(v))).collect::<Vec<_>>().join("&");
    AuthRequest { url: format!("{AUTHORIZE_URL}?{qs}"), verifier, state, nonce, redirect_uri: redirect_uri.into(), client_id, first_registration: first }
}

pub async fn bind_loopback() -> Result<(TcpListener, String), String> {
    let l = match TcpListener::bind(("127.0.0.1", PREFERRED_PORT)).await {
        Ok(l) => l,
        Err(_) => TcpListener::bind(("127.0.0.1", 0)).await.map_err(|e| format!("could not open a local callback port: {e}"))?,
    };
    let port = l.local_addr().map_err(|e| e.to_string())?.port();
    Ok((l, format!("http://127.0.0.1:{port}{CALLBACK_PATH}")))
}

pub struct Callback {
    pub params: HashMap<String, String>,
}

const PAGE_OK: &str = "<!doctype html><meta charset=utf-8><title>InputFlow</title><body style=\"font:15px -apple-system,system-ui,sans-serif;display:grid;place-items:center;height:90vh;color:#222\"><div style=\"text-align:center\"><h2 style=\"font-weight:600\">You can return to InputFlow</h2><p>This tab can be closed.</p></div>";
const PAGE_ERR: &str = "<!doctype html><meta charset=utf-8><title>InputFlow</title><body style=\"font:15px -apple-system,system-ui,sans-serif;display:grid;place-items:center;height:90vh;color:#222\"><div style=\"text-align:center\"><h2 style=\"font-weight:600\">Sign-in was not completed</h2><p>Return to InputFlow for details.</p></div>";

/// Wait for the browser to hit the loopback callback. Ignores unrelated requests.
pub async fn wait_for_callback(listener: TcpListener, expected_state: &str, timeout: Duration) -> Result<Callback, String> {
    let fut = async {
        loop {
            let (mut sock, _) = listener.accept().await.map_err(|e| e.to_string())?;
            let mut buf = vec![0u8; 8192];
            let n = match tokio::time::timeout(Duration::from_secs(5), sock.read(&mut buf)).await {
                Ok(Ok(n)) => n,
                _ => continue,
            };
            let req = String::from_utf8_lossy(&buf[..n]);
            let first = req.lines().next().unwrap_or("");
            let target = first.split_whitespace().nth(1).unwrap_or("");
            let Ok(u) = url::Url::parse(&format!("http://127.0.0.1{target}")) else { continue };
            if !first.starts_with("GET ") || u.path() != CALLBACK_PATH {
                let _ = sock.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
                continue;
            }
            let params: HashMap<String, String> = u.query_pairs().into_owned().collect();
            if params.get("state").map(String::as_str) != Some(expected_state) {
                // Possibly a stale/forged request; do not consume our attempt.
                let _ = respond(&mut sock, PAGE_ERR).await;
                continue;
            }
            let ok = params.contains_key("code");
            let _ = respond(&mut sock, if ok { PAGE_OK } else { PAGE_ERR }).await;
            return Ok(Callback { params });
        }
    };
    tokio::time::timeout(timeout, fut).await.map_err(|_| "Sign-in timed out. Please try again.".to_string())?
}

async fn respond(sock: &mut tokio::net::TcpStream, body: &str) -> std::io::Result<()> {
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    sock.write_all(resp.as_bytes()).await
}

pub async fn exchange_code(http: &reqwest::Client, req: &AuthRequest, code: &str, client_id: &str) -> Result<TokenResponse, String> {
    let form = [
        ("grant_type", "authorization_code"),
        ("client_id", client_id),
        ("code", code),
        ("code_verifier", req.verifier.as_str()),
        ("redirect_uri", req.redirect_uri.as_str()),
        ("resource", RESOURCE),
    ];
    token_request(http, &form).await
}

pub async fn refresh(http: &reqwest::Client, client_id: &str, refresh_token: &str) -> Result<TokenResponse, String> {
    let form = [("grant_type", "refresh_token"), ("client_id", client_id), ("refresh_token", refresh_token), ("resource", RESOURCE)];
    token_request(http, &form).await
}

/// Error strings starting with `oauth:` carry the OAuth error code.
async fn token_request(http: &reqwest::Client, form: &[(&str, &str)]) -> Result<TokenResponse, String> {
    let resp = http.post(TOKEN_URL).form(form).send().await.map_err(|e| format!("network: {}", e.without_url()))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("network: {}", e.without_url()))?;
    if !status.is_success() {
        let err: Option<TokenError> = serde_json::from_str(&text).ok();
        let code = err.as_ref().and_then(|e| e.error.clone()).unwrap_or_else(|| format!("http_{}", status.as_u16()));
        let desc = err.and_then(|e| e.error_description).unwrap_or_default();
        return Err(format!("oauth:{code}:{desc}"));
    }
    serde_json::from_str(&text).map_err(|_| "Unexpected response from the token endpoint.".to_string())
}

pub fn is_terminal_refresh_error(e: &str) -> bool {
    ["invalid_grant", "invalid_refresh_token", "token_expired", "refresh_token_expired", "refresh_token_invalidated", "refresh_token_reused"]
        .iter()
        .any(|c| e.starts_with(&format!("oauth:{c}")))
}

#[derive(Debug, Deserialize)]
struct OidcConfig {
    jwks_uri: Option<String>,
    revocation_endpoint: Option<String>,
}

async fn oidc(http: &reqwest::Client) -> Result<OidcConfig, String> {
    http.get(OIDC_CONFIG_URL).send().await.map_err(|e| format!("network: {}", e.without_url()))?.json().await.map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
struct IdClaims {
    sub: String,
    email: Option<String>,
    name: Option<String>,
    nonce: Option<String>,
}

/// Verify the ID token signature (OpenAI JWKS), issuer, audience, expiry and nonce.
pub async fn verify_id_token(http: &reqwest::Client, id_token: &str, client_id: &str, nonce: Option<&str>) -> Result<Account, String> {
    let cfg = oidc(http).await?;
    let jwks_uri = cfg.jwks_uri.ok_or("OpenID configuration has no jwks_uri")?;
    let jwks: jsonwebtoken::jwk::JwkSet = http.get(&jwks_uri).send().await.map_err(|e| format!("network: {}", e.without_url()))?.json().await.map_err(|e| e.to_string())?;
    let header = jsonwebtoken::decode_header(id_token).map_err(|_| "Malformed ID token")?;
    let jwk = match &header.kid {
        Some(kid) => jwks.find(kid),
        None => jwks.keys.first(),
    }
    .ok_or("ID token signing key not found")?;
    let key = jsonwebtoken::DecodingKey::from_jwk(jwk).map_err(|_| "Unsupported ID token key")?;
    let mut v = jsonwebtoken::Validation::new(header.alg);
    v.set_issuer(&[ISSUER]);
    v.set_audience(&[client_id]);
    v.leeway = 60;
    let data = jsonwebtoken::decode::<IdClaims>(id_token, &key, &v).map_err(|e| format!("ID token verification failed: {e}"))?;
    if let Some(n) = nonce {
        if data.claims.nonce.as_deref() != Some(n) {
            return Err("ID token nonce mismatch".into());
        }
    }
    Ok(Account { sub: data.claims.sub, email: data.claims.email, name: data.claims.name })
}

/// Revoke the refresh token. Returns Ok(true) only on confirmed revocation.
pub async fn revoke(http: &reqwest::Client, client_id: &str, refresh_token: &str) -> Result<bool, String> {
    let cfg = oidc(http).await?;
    let Some(endpoint) = cfg.revocation_endpoint else { return Ok(false) };
    let form = [("token", refresh_token), ("token_type_hint", "refresh_token"), ("client_id", client_id)];
    for attempt in 0..3u32 {
        match http.post(&endpoint).form(&form).send().await {
            Ok(r) if r.status().is_success() => return Ok(true),
            Ok(r) if r.status().is_client_error() => return Ok(false),
            _ => tokio::time::sleep(Duration::from_millis(400 * 2u64.pow(attempt))).await,
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_matches_rfc7636_example() {
        // RFC 7636 Appendix B
        assert_eq!(pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn authorize_url_first_registration_vs_returning() {
        let reg = Registration { ext_agent_host_id: "urn:uuid:1234".into(), client_id: None };
        let a = build_authorize(&reg, "http://127.0.0.1:1455/auth/callback", Some("tok"));
        assert!(a.first_registration);
        let u = url::Url::parse(&a.url).unwrap();
        let q: HashMap<_, _> = u.query_pairs().into_owned().collect();
        assert_eq!(u.as_str().split('?').next().unwrap(), AUTHORIZE_URL);
        assert_eq!(q["client_id"], DYNAMIC_CLIENT);
        assert_eq!(q["agent_name_hint"], "InputFlow");
        assert!(!q.contains_key("id_token_hint"), "no id_token_hint during first registration");
        assert_eq!(q["scope"], SCOPES);
        assert_eq!(q["code_challenge_method"], "S256");
        assert_eq!(q["code_challenge"], pkce_challenge(&a.verifier));
        assert_eq!(q["resource"], RESOURCE);
        assert!(a.verifier.len() >= 43);

        let reg2 = Registration { ext_agent_host_id: "urn:uuid:1234".into(), client_id: Some("oaiapp_x".into()) };
        let b = build_authorize(&reg2, "http://127.0.0.1:5555/auth/callback", Some("tok"));
        let q2: HashMap<_, _> = url::Url::parse(&b.url).unwrap().query_pairs().into_owned().collect();
        assert_eq!(q2["client_id"], "oaiapp_x");
        assert!(!q2.contains_key("agent_name_hint"));
        assert_eq!(q2["id_token_hint"], "tok");
        assert_ne!(a.state, b.state);
    }

    #[test]
    fn terminal_refresh_errors() {
        assert!(is_terminal_refresh_error("oauth:invalid_grant:"));
        assert!(is_terminal_refresh_error("oauth:refresh_token_reused:x"));
        assert!(!is_terminal_refresh_error("network: timeout"));
        assert!(!is_terminal_refresh_error("oauth:http_503:"));
    }

    #[tokio::test]
    async fn loopback_callback_validates_state_and_path() {
        let (l, redirect) = bind_loopback().await.unwrap();
        let port = url::Url::parse(&redirect).unwrap().port().unwrap();
        let waiter = tokio::spawn(wait_for_callback(l, "good", Duration::from_secs(5)));
        let get = |path: String| async move {
            let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
            s.write_all(format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes()).await.unwrap();
            let mut b = String::new();
            let _ = s.read_to_string(&mut b).await;
            b
        };
        assert!(get("/other".into()).await.starts_with("HTTP/1.1 404"));
        get("/auth/callback?code=c&state=evil".into()).await;
        get("/auth/callback?code=abc&state=good&client_id=oaiapp_1".into()).await;
        let cb = waiter.await.unwrap().unwrap();
        assert_eq!(cb.params["code"], "abc");
        assert_eq!(cb.params["client_id"], "oaiapp_1");
    }
}
