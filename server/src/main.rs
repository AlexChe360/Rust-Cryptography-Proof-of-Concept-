use axum::{
    Json, Router,
    extract::State,
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use dashmap::DashMap;
use ed25519_dalek::{Signature, SigningKey, Verifier, VerifyingKey};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tower_http::cors::{Any, CorsLayer};

// --------------
// POC - config
// --------------

const HARCODED_CODE: &str = "123456";
const VERIFICATION_TTL: Duration = Duration::from_secs(300); // 5 minutes
const TEMP_CREDENTIAL_TTL: Duration = Duration::from_secs(300);
const SESSION_TTL: Duration = Duration::from_secs(1800); // 30 minutes

// -------------
// In-memory state
// -------------

#[derive(Clone)]
struct AppState {
    verification_tokens: Arc<DashMap<String, VerificationTokenRecord>>,
    temporary_credentials: Arc<DashMap<String, TemporaryCredentialRecord>>,
    sessions: Arc<DashMap<String, SessionRecord>>,
}

#[derive(Clone)]
struct VerificationTokenRecord {
    expires_at: Instant,
}

#[derive(Clone)]
struct TemporaryCredentialRecord {
    public_key: VerifyingKey,
    expires_at: Instant,
}

#[derive(Clone)]
struct SessionRecord {
    expires_at: Instant,
}

// -------------
// DTO
// -------------

#[derive(Deserialize)]
struct VerifyUseRequest {
    username: String,
    code: String,
}

#[derive(Serialize)]
struct VerifyUserResponse {
    verification_token: String,
    expires_in_seconds: u64,
}

#[derive(Deserialize)]
struct IssueTemporaryCredentialsRequest {
    verification_token: String,
}

#[derive(Serialize)]
struct IssueTemporaryCredentialsResponse {
    credential_id: String,
    credential_private: String,
    expires_in_seconds: u64,
}

#[derive(Deserialize)]
struct EnterSessionRequest {
    credential_id: String,
    message: String,
    signature: String,
}

#[derive(Serialize)]
struct EnterSessionResponse {
    session_token: String,
    expires_in_seconds: u64,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

// ------------
// Utils
// ------------

fn random_token(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    OsRng.fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

fn deadline(ttl: Duration) -> Instant {
    Instant::now() + ttl
}

fn expired(t: Instant) -> bool {
    Instant::now() > t
}

fn json_error(status: StatusCode, msg: &str) -> Response {
    (status, Json(ErrorResponse { error: msg.into() })).into_response()
}

fn json_ok<T: Serialize>(status: StatusCode, body: T) -> Response {
    (status, Json(body)).into_response()
}

// ------------
// Real
// ------------

async fn verify_user(State(state): State<AppState>, Json(req): Json<VerifyUseRequest>) -> Response {
    let username = req.username.trim().to_string();
    if username.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "username_required");
    }

    if req.code != HARCODED_CODE {
        return json_error(StatusCode::UNAUTHORIZED, "invalid code");
    }

    let token = random_token(32);

    state.verification_tokens.insert(
        token.clone(),
        VerificationTokenRecord {
            expires_at: deadline(VERIFICATION_TTL),
        },
    );

    json_ok(
        StatusCode::OK,
        VerifyUserResponse {
            verification_token: token,
            expires_in_seconds: VERIFICATION_TTL.as_secs(),
        },
    )
}

async fn issue_temporary_credentials(
    State(state): State<AppState>,
    Json(req): Json<IssueTemporaryCredentialsRequest>,
) -> Response {
    let token = req.verification_token.trim();
    if token.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "verification_token_required");
    }

    let rec = match state.verification_tokens.get(token) {
        Some(v) => v,
        None => {
            return json_error(
                StatusCode::UNAUTHORIZED,
                "invalid_or_expired_verification_token",
            );
        }
    };

    if expired(rec.expires_at) {
        state.verification_tokens.remove(token);
        return json_error(
            StatusCode::UNAUTHORIZED,
            "invalid_or_expired_verification_token",
        );
    }

    // Generation Ed25519
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    // Identificator record on server
    let credential_id = random_token(24);

    // Private key client
    let private_seed = signing_key.to_bytes();
    let private_b64 = URL_SAFE_NO_PAD.encode(private_seed);

    state.temporary_credentials.insert(
        credential_id.clone(),
        TemporaryCredentialRecord {
            public_key: verifying_key,
            expires_at: deadline(TEMP_CREDENTIAL_TTL),
        },
    );

    json_ok(
        StatusCode::OK,
        IssueTemporaryCredentialsResponse {
            credential_id,
            credential_private: private_b64,
            expires_in_seconds: TEMP_CREDENTIAL_TTL.as_secs(),
        },
    )
}

async fn enter_session_with_credential(
    State(state): State<AppState>,
    Json(req): Json<EnterSessionRequest>,
) -> Response {
    let credential_id = req.credential_id.trim();
    if credential_id.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "credential_id_required");
    }
    if req.message.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "message_required");
    }
    if req.signature.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "signature_required");
    }

    let cred = match state.temporary_credentials.get(credential_id) {
        Some(v) => v,
        None => {
            return json_error(StatusCode::UNAUTHORIZED, "invalid_or_expired_credential");
        }
    };

    if expired(cred.expires_at) {
        state.temporary_credentials.remove(credential_id);
        return json_error(StatusCode::UNAUTHORIZED, "invalid_or_expired_credential");
    }

    let sig_bytes = match URL_SAFE_NO_PAD.decode(req.signature.as_bytes()) {
        Ok(b) => b,
        Err(_) => {
            return json_error(StatusCode::BAD_REQUEST, "signature_not_base64url");
        }
    };

    let signature = match Signature::from_slice(&sig_bytes) {
        Ok(s) => s,
        Err(_) => {
            return json_error(StatusCode::BAD_REQUEST, "signature_invalid_format");
        }
    };

    if cred
        .public_key
        .verify(req.message.as_bytes(), &signature)
        .is_err()
    {
        return json_error(StatusCode::UNAUTHORIZED, "invalid_signature");
    }

    let session_token = random_token(32);
    state.sessions.insert(
        session_token.clone(),
        SessionRecord {
            expires_at: deadline(SESSION_TTL),
        },
    );

    json_ok(
        StatusCode::OK,
        EnterSessionResponse {
            session_token,
            expires_in_seconds: SESSION_TTL.as_secs(),
        },
    )
}

async fn submit_user_preferences(Json(obj): Json<Value>) -> Response {
    let map = match obj.as_object() {
        Some(m) => m,
        None => {
            return json_error(StatusCode::BAD_REQUEST, "preferences_must_be_object");
        }
    };

    if map.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "preferences_empty");
    }

    for k in map.keys() {
        if k.trim().is_empty() {
            return json_error(StatusCode::BAD_REQUEST, "invalid_preference_key");
        }
    }

    json_ok(
        StatusCode::OK,
        serde_json::json!({
            "ok": true,
            "prefernce": obj
        }),
    )
}

// ------------
// Clear expired state
// ------------

async fn cleanup_expired_state(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        let now = Instant::now();

        state.verification_tokens.retain(|_, v| v.expires_at > now);
        state
            .temporary_credentials
            .retain(|_, v| v.expires_at > now);
        state.sessions.retain(|_, v| v.expires_at > now);
    }
}

// --------------
// Main
// --------------

#[tokio::main]
async fn main() {
    let state = AppState {
        verification_tokens: Arc::new(DashMap::new()),
        temporary_credentials: Arc::new(DashMap::new()),
        sessions: Arc::new(DashMap::new()),
    };

    tokio::spawn(cleanup_expired_state(state.clone()));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::POST])
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/step1/verify", post(verify_user))
        .route(
            "/api/step2/issue-credentials",
            post(issue_temporary_credentials),
        )
        .route("/api/step3/enter", post(enter_session_with_credential))
        .route("/api/user/preferences", post(submit_user_preferences))
        .layer(cors)
        .with_state(state);

    let addr = "0.0.0.0:8080";
    println!("Rust Cryptograph POC running on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
