# Rust Cryptography Proof of Concept — Staged Access (In-Memory)

A lightweight Rust backend that simulates a **three-stage user access workflow** for research and prototyping.

This project is a **POC** and **not a production system**.

---

## What This POC Does

The backend exposes three sequential endpoints that emulate a staged login/access pipeline:

1. **User verification** (hardcoded one-time code)
2. **Temporary credential issuance**
3. **Credential-based session entry**

State is held **in memory only** (no database).

A small preferences endpoint is included to simulate basic user settings handling.

---

## Key Characteristics

- **No database, no persistence**
- **In-memory state** using simple maps
- **Short-lived tokens/credentials** with TTL
- **Ed25519-based proof of possession**
  - client receives the temporary **private** part (seed)
  - server stores the **public** counterpart

---

## Project Structure

This repository uses a **Cargo workspace**:

```text
rust-crypto-poc/
├── server/
│   └── src/main.rs
├── client/
│   └── src/main.rs
├── Cargo.toml
├── Cargo.lock
├── README.md
└── .gitignore

```
- **server** — Axum 0.7 backend
- **client** — minimal Rust script demonstrating the full 3-step flow

---

## Requirements

- **Rust stable** (recent version recommended)
- **Cargo**

---

## Dependencies Notes

This POC requires RNG support for Ed25519 key generation.

In `server/Cargo.toml`:

```toml
ed25519-dalek = { version = "2", features = ["rand_core"] }
```

---

## How to Run

```bash
# terminal 1
cargo run -p staged-access-server

# terminal 2
cargo run -p staged-access-client

```


## API Reference


```yaml
api_reference:
  - name: User Verification
    method: POST
    path: /api/step1/verify
    description: Simulates preliminary user verification.
    request:
      content_type: application/json
      body:
        username: alice
        code: "123456"
    response_200:
      verification_token: "base64url..."
      expires_in_seconds: 300
    errors:
      - status: 400
        code: username_required
      - status: 401
        code: invalid_code

  - name: Temporary Credential Issuance
    method: POST
    path: /api/step2/issue-credentials
    description: >
      Generates a temporary Ed25519 keypair.
      Returns the private seed to the client.
      Stores the public key in memory for validation in Step 3.
    request:
      content_type: application/json
      body:
        verification_token: "base64url..."
    response_200:
      credential_id: "base64url..."
      credential_private: "base64url(seed32)"
      expires_in_seconds: 300
    notes:
      - credential_private is a 32-byte Ed25519 seed encoded with base64url.
      - The client reconstructs the signing key from this seed.
    errors:
      - status: 400
        code: verification_token_required
      - status: 401
        code: invalid_or_expired_verification_token

  - name: Credential-Based Session Entry
    method: POST
    path: /api/step3/enter
    description: Validates that the client possesses the issued temporary credential.
    request:
      content_type: application/json
      body:
        credential_id: "base64url..."
        message: "hello-proof"
        signature: "base64url(signature)"
    response_200:
      session_token: "base64url..."
      expires_in_seconds: 1800
    errors:
      - status: 400
        code: credential_id_required
      - status: 400
        code: message_required
      - status: 400
        code: signature_required
      - status: 400
        code: signature_not_base64url
      - status: 400
        code: signature_invalid_format
      - status: 401
        code: invalid_or_expired_credential
      - status: 401
        code: invalid_signature

  - name: Preferences (No Storage)
    method: POST
    path: /api/user/preferences
    description: >
      Accepts a small JSON object representing generic user settings.
      No persistence — validates basic shape and echoes the payload.
    request:
      content_type: application/json
      body:
        theme: dark
        notifications: true
    response_200:
      ok: true
      preferences:
        theme: dark
        notifications: true
    errors:
      - status: 400
        code: preferences_must_be_object
      - status: 400
        code: preferences_empty
      - status: 400
        code: invalid_preference_key
```
