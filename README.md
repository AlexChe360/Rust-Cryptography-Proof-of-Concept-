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

rust-crypto-poc/
Cargo.toml
Cargo.lock
README.md
.gitignore
server/
Cargo.toml
src/main.rs
client/
Cargo.toml
src/main.rs



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


---

## How to Run
