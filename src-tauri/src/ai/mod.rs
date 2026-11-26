//! ChatGPT connection: account state, token lifecycle, model list and the bounded
//! text-transformation call (`POST /v1/responses`, `store:false`, `stream:true`).
//!
//! Design decision: we use the documented *direct inference* route rather than the
//! Codex app-server. Writing actions are single, tool-less text transformations;
//! the app-server is an agent runtime with local shell/MCP tools, which is the
//! opposite of the isolation MinPoint requires for selected text.

pub mod auth;

use auth::{Account, Registration, TokenSet};
use futures_util::StreamExt;
use minpoint_core::writing::AiRequest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::Mutex as AsyncMutex;

const API: &str = "https://api.openai.com/v1";
const CRED_ACCOUNT: &str = "chatgpt-tokens";
pub const MANAGE_USAGE_URL: &str = "https://chatgpt.com/settings/usage";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ConnState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiStatus {
    pub state: ConnState,
    pub email: Option<String>,
    pub name: Option<String>,
    /// `chatgpt.tokens.use.direct` was granted.
    pub plan_usage_authorized: bool,
    pub usage_limited: bool,
    pub error: Option<String>,
    pub models: Vec<ModelInfo>,
    pub manage_usage_url: &'static str,
    pub show_welcome: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub slug: String,
    pub display_name: String,
}

/// User-facing AI error with a machine-readable kind for the UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiError {
    pub kind: &'static str,
    pub message: String,
}

impl AiError {
    fn new(kind: &'static str, message: impl Into<String>) -> Self {
        AiError { kind, message: message.into() }
    }
}

pub struct AiService {
    http: reqwest::Client,
    reg_path: PathBuf,
    tokens: AsyncMutex<Option<TokenSet>>,
    status: Mutex<AiStatus>,
    cancel_login: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    welcome_path: PathBuf,
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

impl AiService {
    pub fn new(config_dir: &std::path::Path) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("MinPoint/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("http client");
        let svc = AiService {
            http,
            reg_path: config_dir.join("chatgpt-registration.json"),
            welcome_path: config_dir.join(".chatgpt-welcome-shown"),
            tokens: AsyncMutex::new(None),
            status: Mutex::new(AiStatus {
                state: ConnState::Disconnected,
                email: None,
                name: None,
                plan_usage_authorized: false,
                usage_limited: false,
                error: None,
                models: vec![],
                manage_usage_url: MANAGE_USAGE_URL,
                show_welcome: false,
            }),
            cancel_login: Mutex::new(None),
        };
        svc
    }

    /// Load a saved sign-in from the credential store. Called from a background
    /// thread after startup so a slow or prompting credential store can't block the UI.
    pub fn load_stored(&self) {
        match crate::secrets::get(CRED_ACCOUNT) {
            Ok(Some(raw)) => match serde_json::from_str::<TokenSet>(&raw) {
                Ok(t) => {
                    self.apply_connected(&t);
                    *self.tokens.blocking_lock() = Some(t);
                    log::info!("saved ChatGPT sign-in loaded");
                }
                Err(_) => log::warn!("saved ChatGPT sign-in is unreadable; ignoring it"),
            },
            Ok(None) => {}
            Err(e) => log::warn!("credential store unavailable: {e}"),
        }
    }

    pub fn status(&self) -> AiStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn is_ready(&self) -> bool {
        let s = self.status.lock().unwrap();
        s.state == ConnState::Connected && s.plan_usage_authorized
    }

    fn apply_connected(&self, t: &TokenSet) {
        let mut s = self.status.lock().unwrap();
        s.state = ConnState::Connected;
        s.email = t.account.email.clone();
        s.name = t.account.name.clone();
        s.plan_usage_authorized = t.scope.split_whitespace().any(|x| x == auth::PLAN_SCOPE);
        s.error = if s.plan_usage_authorized { None } else { Some("Plan usage was not authorized. Reconnect and allow MinPoint to use your ChatGPT plan.".into()) };
        s.show_welcome = s.plan_usage_authorized && !self.welcome_path.exists();
    }

    pub fn dismiss_welcome(&self) {
        let _ = std::fs::write(&self.welcome_path, b"1");
        self.status.lock().unwrap().show_welcome = false;
    }

    fn set_error(&self, state: ConnState, msg: impl Into<String>) {
        let mut s = self.status.lock().unwrap();
        s.state = state;
        s.error = Some(msg.into());
    }

    fn registration(&self) -> Registration {
        let mut r: Registration = std::fs::read_to_string(&self.reg_path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        if r.ext_agent_host_id.is_empty() {
            r.ext_agent_host_id = format!("urn:uuid:{}", uuid::Uuid::new_v4());
            self.save_registration(&r);
        }
        r
    }

    fn save_registration(&self, r: &Registration) {
        let _ = crate::util::write_atomic(&self.reg_path, serde_json::to_string_pretty(r).unwrap().as_bytes());
    }

    fn store_tokens(&self, t: &TokenSet) -> Result<(), String> {
        crate::secrets::set(CRED_ACCOUNT, &serde_json::to_string(t).map_err(|e| e.to_string())?)
    }

    pub fn cancel_connect(&self) {
        if let Some(tx) = self.cancel_login.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }

    /// Run the full sign-in. `open_url` opens the system browser.
    pub async fn connect(&self, open_url: impl Fn(&str) -> Result<(), String>) -> Result<AiStatus, String> {
        self.cancel_connect();
        {
            let mut s = self.status.lock().unwrap();
            s.state = ConnState::Connecting;
            s.error = None;
        }
        let r = self.connect_inner(open_url).await;
        match &r {
            Ok(()) => {}
            Err(e) => {
                let had_tokens = self.tokens.lock().await.is_some();
                self.set_error(if had_tokens { ConnState::Connected } else { ConnState::Error }, e.clone());
                if !had_tokens {
                    self.status.lock().unwrap().state = ConnState::Error;
                }
            }
        }
        r.map(|_| self.status())
    }

    async fn connect_inner(&self, open_url: impl Fn(&str) -> Result<(), String>) -> Result<(), String> {
        let mut reg = self.registration();
        let hint = self.tokens.lock().await.as_ref().and_then(|t| t.id_token.clone());
        let (listener, redirect) = auth::bind_loopback().await?;
        let req = auth::build_authorize(&reg, &redirect, hint.as_deref());
        let (tx, rx) = tokio::sync::oneshot::channel();
        *self.cancel_login.lock().unwrap() = Some(tx);
        open_url(&req.url)?;
        let cb = tokio::select! {
            r = auth::wait_for_callback(listener, &req.state, Duration::from_secs(300)) => r?,
            _ = rx => return Err("Sign-in cancelled.".into()),
        };
        if let Some(err) = cb.params.get("error") {
            return Err(if err == "access_denied" { "You declined the authorization request.".into() } else { format!("Sign-in failed ({err}).") });
        }
        let code = cb.params.get("code").ok_or("Sign-in did not return an authorization code.")?;
        let client_id = if req.first_registration {
            let issued = cb.params.get("client_id").filter(|c| c.as_str() != auth::DYNAMIC_CLIENT).cloned().ok_or("Registration incomplete: no client ID was issued. Please try again.")?;
            reg.client_id = Some(issued.clone());
            self.save_registration(&reg);
            issued
        } else {
            req.client_id.clone()
        };
        let tr = match auth::exchange_code(&self.http, &req, code, &client_id).await {
            Ok(t) => t,
            Err(e) if e.starts_with("oauth:invalid_grant") => return Err("The authorization code expired. Please connect again.".into()),
            Err(e) => return Err(friendly_oauth(&e)),
        };
        let account = match &tr.id_token {
            Some(id) => auth::verify_id_token(&self.http, id, &client_id, Some(&req.nonce)).await?,
            None => Account::default(),
        };
        let t = TokenSet {
            client_id,
            access_token: tr.access_token,
            refresh_token: tr.refresh_token,
            id_token: tr.id_token,
            expires_at: now() + tr.expires_in.unwrap_or(3600),
            scope: tr.scope.unwrap_or_default(),
            account,
        };
        self.store_tokens(&t)?;
        self.apply_connected(&t);
        *self.tokens.lock().await = Some(t);
        let _ = self.refresh_models().await;
        Ok(())
    }

    /// Return a valid access token, refreshing (serialised) when close to expiry.
    async fn access_token(&self) -> Result<String, AiError> {
        let mut guard = self.tokens.lock().await;
        let t = guard.as_mut().ok_or_else(|| AiError::new("not_connected", "Connect your ChatGPT account in MinPoint → Connected Account."))?;
        if t.expires_at - now() > 120 {
            return Ok(t.access_token.clone());
        }
        let rt = t.refresh_token.clone().ok_or_else(|| AiError::new("reauth", "Your ChatGPT session expired. Please reconnect."))?;
        match auth::refresh(&self.http, &t.client_id, &rt).await {
            Ok(tr) => {
                t.access_token = tr.access_token;
                if tr.refresh_token.is_some() {
                    t.refresh_token = tr.refresh_token;
                }
                if tr.id_token.is_some() {
                    t.id_token = tr.id_token;
                }
                if let Some(sc) = tr.scope {
                    t.scope = sc;
                }
                t.expires_at = now() + tr.expires_in.unwrap_or(3600);
                let _ = self.store_tokens(t);
                let snapshot = t.clone();
                drop(guard);
                self.apply_connected(&snapshot);
                Ok(snapshot.access_token)
            }
            Err(e) if auth::is_terminal_refresh_error(&e) => {
                *guard = None;
                drop(guard);
                let _ = crate::secrets::delete(CRED_ACCOUNT);
                self.set_error(ConnState::Error, "Your ChatGPT authorization ended. Please reconnect.");
                Err(AiError::new("reauth", "Your ChatGPT authorization ended. Please reconnect."))
            }
            Err(e) if e.starts_with("network") => Err(AiError::new("network", "No connection to ChatGPT. Check your internet connection.")),
            Err(e) => Err(AiError::new("auth", friendly_oauth(&e))),
        }
    }

    pub async fn disconnect(&self) -> Result<bool, String> {
        self.cancel_connect();
        let t = self.tokens.lock().await.take();
        let mut confirmed = true;
        if let Some(t) = &t {
            if let Some(rt) = &t.refresh_token {
                confirmed = auth::revoke(&self.http, &t.client_id, rt).await.unwrap_or(false);
            }
        }
        crate::secrets::delete(CRED_ACCOUNT)?;
        let mut s = self.status.lock().unwrap();
        s.state = ConnState::Disconnected;
        s.email = None;
        s.name = None;
        s.plan_usage_authorized = false;
        s.usage_limited = false;
        s.models.clear();
        s.error = if confirmed { None } else { Some("Local credentials were removed, but revocation could not be confirmed. You can also disconnect MinPoint in ChatGPT Settings.".into()) };
        Ok(confirmed)
    }

    pub async fn refresh_models(&self) -> Result<Vec<ModelInfo>, AiError> {
        let tok = self.access_token().await?;
        let resp = self.http.get(format!("{API}/models")).bearer_auth(tok).timeout(Duration::from_secs(20)).send().await.map_err(net)?;
        let status = resp.status();
        let body: Value = resp.json().await.map_err(net)?;
        if !status.is_success() {
            return Err(map_api_error(status.as_u16(), &body));
        }
        let arr = body.get("models").or_else(|| body.get("data")).and_then(Value::as_array).cloned().unwrap_or_default();
        let models: Vec<ModelInfo> = arr
            .iter()
            .filter(|m| m.get("visibility").and_then(Value::as_str).map_or(true, |v| v == "list"))
            .filter_map(|m| {
                let slug = m.get("slug").or_else(|| m.get("id")).and_then(Value::as_str)?.to_string();
                let display_name = m.get("display_name").and_then(Value::as_str).unwrap_or(&slug).to_string();
                Some(ModelInfo { slug, display_name })
            })
            .collect();
        self.status.lock().unwrap().models = models.clone();
        Ok(models)
    }

    /// Stream a bounded transformation. `on_delta` receives text chunks as they arrive.
    pub async fn transform(&self, model: Option<&str>, req: &AiRequest, mut on_delta: impl FnMut(&str), cancel: tokio::sync::watch::Receiver<bool>) -> Result<String, AiError> {
        if !self.status.lock().unwrap().plan_usage_authorized {
            return Err(AiError::new("plan_not_authorized", "MinPoint is not authorized to use your ChatGPT plan. Reconnect to allow it."));
        }
        let model = match model {
            Some(m) => m.to_string(),
            None => {
                let ms = { self.status.lock().unwrap().models.clone() };
                let ms = if ms.is_empty() { self.refresh_models().await? } else { ms };
                ms.first().map(|m| m.slug.clone()).ok_or_else(|| AiError::new("no_models", "No models are available for this account."))?
            }
        };
        let tok = self.access_token().await?;
        // Only fields documented as supported for plan usage. No tools, no storage.
        let body = json!({
            "model": model,
            "instructions": req.instructions,
            "input": [{ "role": "user", "content": req.input }],
            "store": false,
            "stream": true,
        });
        let mut attempt = 0;
        let resp = loop {
            let r = self.http.post(format!("{API}/responses")).bearer_auth(&tok).json(&body).timeout(Duration::from_secs(120)).send().await.map_err(net)?;
            let st = r.status().as_u16();
            if st == 503 && attempt < 2 {
                attempt += 1;
                tokio::time::sleep(Duration::from_millis(800 * attempt)).await;
                continue;
            }
            if !r.status().is_success() {
                let v: Value = r.json().await.unwrap_or(Value::Null);
                let e = map_api_error(st, &v);
                self.note_error(&e);
                return Err(e);
            }
            break r;
        };
        let mut stream = resp.bytes_stream();
        let mut buf = String::new();
        let mut out = String::new();
        let mut completed = false;
        let mut cancel = cancel;
        loop {
            let chunk = tokio::select! {
                c = stream.next() => c,
                _ = cancel.changed() => return Err(AiError::new("cancelled", "Cancelled.")),
            };
            let Some(chunk) = chunk else { break };
            let chunk = chunk.map_err(net)?;
            buf.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(pos) = buf.find("\n\n") {
                let frame = buf[..pos].to_string();
                buf.drain(..pos + 2);
                for line in frame.lines() {
                    let Some(data) = line.strip_prefix("data:") else { continue };
                    let data = data.trim();
                    if data == "[DONE]" {
                        continue;
                    }
                    let Ok(ev) = serde_json::from_str::<Value>(data) else { continue };
                    match ev.get("type").and_then(Value::as_str).unwrap_or("") {
                        "response.output_text.delta" => {
                            if let Some(d) = ev.get("delta").and_then(Value::as_str) {
                                out.push_str(d);
                                on_delta(d);
                            }
                        }
                        "response.completed" => completed = true,
                        "response.failed" | "error" => {
                            let err = ev.pointer("/response/error").or_else(|| ev.get("error")).cloned().unwrap_or(Value::Null);
                            let e = map_api_error(0, &json!({ "error": err }));
                            self.note_error(&e);
                            return Err(e);
                        }
                        "response.incomplete" => return Err(AiError::new("incomplete", "The response was cut short. Try a shorter selection or regenerate.")),
                        _ => {}
                    }
                }
            }
        }
        if !completed {
            return Err(AiError::new("interrupted", "The connection was interrupted before the response completed."));
        }
        self.status.lock().unwrap().usage_limited = false;
        Ok(out)
    }

    fn note_error(&self, e: &AiError) {
        if e.kind == "usage_limit" {
            self.status.lock().unwrap().usage_limited = true;
        }
    }
}

fn net(e: reqwest::Error) -> AiError {
    if e.is_timeout() {
        AiError::new("network", "ChatGPT took too long to respond. Try again.")
    } else {
        AiError::new("network", "Could not reach ChatGPT. Check your internet connection.")
    }
}

fn friendly_oauth(e: &str) -> String {
    if e.starts_with("network") {
        return "Could not reach OpenAI. Check your internet connection.".into();
    }
    if e.starts_with("oauth:invalid_client") {
        return "OpenAI rejected MinPoint's client registration. Disconnect and connect again.".into();
    }
    let code = e.split(':').nth(1).unwrap_or("unknown");
    format!("Authorization failed ({code}).")
}

/// Map documented plan-usage errors to user-facing messages.
pub fn map_api_error(status: u16, body: &Value) -> AiError {
    let code = body.pointer("/error/code").and_then(Value::as_str).unwrap_or("");
    let param = body.pointer("/error/param").and_then(Value::as_str).unwrap_or("");
    match code {
        "subscription_sharing_usage_limit_exceeded" => AiError::new("usage_limit", "You've reached your ChatGPT plan's usage limit. Manage usage in ChatGPT settings."),
        "subscription_sharing_user_not_eligible" => AiError::new("not_eligible", "This ChatGPT account isn't eligible to use its plan in third-party apps."),
        "subscription_sharing_usage_unavailable" | "subscription_sharing_user_unavailable" => AiError::new("unavailable", "ChatGPT plan usage is temporarily unavailable. Try again shortly."),
        "subscription_sharing_unsupported_capability" => AiError::new("unsupported", format!("This request uses an unsupported option ({param}).")),
        "subscription_sharing_invalid_user" => AiError::new("reauth", "Your ChatGPT authorization is no longer valid. Please reconnect."),
        "subscription_sharing_route_not_supported" | "chatpass_v2_scope_not_authorized" | "chatpass_v2_invalid_authorization_context" => {
            AiError::new("config", "OpenAI rejected the request configuration. Please reconnect; if it persists, report an MinPoint bug.")
        }
        _ => match status {
            401 => AiError::new("reauth", "ChatGPT rejected the credentials. Please reconnect."),
            403 => AiError::new("forbidden", "OpenAI blocked this request (policy or region restriction)."),
            429 => AiError::new("usage_limit", "Rate or usage limit reached. Manage usage in ChatGPT settings."),
            503 => AiError::new("unavailable", "ChatGPT is temporarily unavailable. Try again shortly."),
            404 => AiError::new("model", "The selected model is unavailable. Choose another model in Connected Account."),
            _ => AiError::new("api", body.pointer("/error/message").and_then(Value::as_str).unwrap_or("ChatGPT returned an error.").to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_documented_errors() {
        let e = |c: &str| map_api_error(0, &json!({"error": {"code": c}})).kind;
        assert_eq!(e("subscription_sharing_usage_limit_exceeded"), "usage_limit");
        assert_eq!(e("subscription_sharing_user_not_eligible"), "not_eligible");
        assert_eq!(e("subscription_sharing_usage_unavailable"), "unavailable");
        assert_eq!(e("subscription_sharing_invalid_user"), "reauth");
        assert_eq!(map_api_error(403, &Value::Null).kind, "forbidden");
        assert_eq!(map_api_error(401, &json!({"detail": "x"})).kind, "reauth");
    }
}
