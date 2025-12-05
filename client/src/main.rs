

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ed25519_dalek::{Signer, SigningKey, Signature};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const BASE: &str = "http://localhost:8080";

// -------- DTO клиента --------

#[derive(Serialize)]
struct VerifyUserRequest {
    username: String,
    code: String,
}

#[derive(Deserialize)]
struct VerifyUserResponse {
    verification_token: String,
}

#[derive(Serialize)]
struct IssueTemporaryCredentialsRequest {
    verification_token: String,
}

#[derive(Deserialize)]
struct IssueTemporaryCredentialsResponse {
    credential_id: String,
    credential_private: String,
}

#[derive(Serialize)]
struct EnterSessionRequest {
    credential_id: String,
    message: String,
    signature: String,
}

#[derive(Deserialize)]
struct EnterSessionResponse {
    session_token: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let http = Client::new();

    // 1) verify
    let v: VerifyUserResponse = http
        .post(format!("{BASE}/api/step1/verify"))
        .json(&VerifyUserRequest {
            username: "alice".into(),
            code: "123456".into(),
        })
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    println!("verification_token: {}", v.verification_token);

    // 2) issue temporary credentials
    let c: IssueTemporaryCredentialsResponse = http
        .post(format!("{BASE}/api/step2/issue-credentials"))
        .json(&IssueTemporaryCredentialsRequest {
            verification_token: v.verification_token.clone(),
        })
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    println!("credential_id: {}", c.credential_id);
    println!("credential_private (client-held): {}", c.credential_private);

    // reconstruct SigningKey from seed(32 bytes)
    let seed_bytes = URL_SAFE_NO_PAD.decode(c.credential_private.as_bytes())?;
    let seed: [u8; 32] = seed_bytes
        .try_into()
        .map_err(|_| "invalid private key length")?;
    let signing_key = SigningKey::from_bytes(&seed);

    // 3) sign + enter session
    let message = "hello-proof";
    let sig: Signature = signing_key.sign(message.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig.to_bytes());

    let s: EnterSessionResponse = http
        .post(format!("{BASE}/step3/enter"))
        .json(&EnterSessionRequest {
            credential_id: c.credential_id.clone(),
            message: message.into(),
            signature: sig_b64,
        })
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    println!("session_token: {}", s.session_token);

    // 4) preferences
    let pref = http
        .post(format!("{BASE}/api/user/preferences"))
        .json(&serde_json::json!({
            "theme": "dark",
            "notifications": true
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;

    println!("preferences response: {pref}");

    println!("\nFlow complete ✅");
    Ok(())
}
