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

This POC simulates a three-stage access workflow using in-memory state and Ed25519 proof-of-possession.

### Overview

| Stage | Endpoint | Purpose |
|------:|----------|---------|
| 1 | `POST /api/step1/verify` | Simulated user verification (hardcoded code) |
| 2 | `POST /api/step2/issue-credentials` | Issue temporary Ed25519-based credentials |
| 3 | `POST /api/step3/enter` | Verify proof-of-possession and return a session token |
| — | `POST /api/user/preferences` | Minimal preferences validation (no storage) |

---

### 1) User Verification

**POST** `/api/step1/verify`  
Simulates preliminary user verification using a hardcoded one-time code.

**Request**
```json
{
  "username": "alice",
  "code": "123456"
}
```

**Response 200**
```json
{
  "verification_token": "base64url...",
  "expires_in_seconds": 300
}
```

**Errors**
- **400 username_required**
- **401 invalid_code**

---

### 2) Temporary Credential Issuance

**POST** `/api/step2/issue-credentials`
Generates a temporary Ed25519 keypair.

- **Returns the private seed to the client.**
- **Stores the public key in memory for Step 3 validation.**

**Request**
```json
{
  "verification_token": "base64url..."
}
```

**Response 200**
```json
{
  "credential_id": "base64url...",
  "credential_private": "base64url(seed32)",
  "expires_in_seconds": 300
}
```
**Notes**
- **credential_private is a 32-byte Ed25519 seed encoded with base64url.**
- **The client reconstructs the signing key from this seed.**

**Errors**
- **400 verification_token_required**
- **401 invalid_or_expired_verification_token**

---

### 3) Credential-Based Session Entry

**POST** `/api/step3/enter`
Validates that the client possesses the issued temporary credential.

**Request**
```json
{
  "credential_id": "base64url...",
  "message": "hello-proof",
  "signature": "base64url(signature)"
}
```

**Response 200**
```json
{
  "session_token": "base64url...",
  "expires_in_seconds": 1800
}
```

**Errors**
- **400 credential_id_required**
- **400 message_required**
- **400 signature_required**
- **400 signature_not_base64url**
- **400 signature_invalid_format**
- **401 invalid_or_expired_credential**
- **401 invalid_signature**

---

### 4) Preferences (No Storage)

**POST** `/api/user/preferences`
Accepts a small JSON object representing generic user settings.
No persistence — validates basic shape and echoes the payload.

**Request**
```json
{
  "theme": "dark",
  "notifications": true
}
```

**Response 200**
```json
{
  "ok": true,
  "preferences": {
    "theme": "dark",
    "notifications": true
  }
}
```

**Errors**
- **400 preferences_must_be_object**
- **400 preferences_empty**
- **400 invalid_preference_key**