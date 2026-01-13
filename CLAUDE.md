# CLAUDE.md - Crab Project Guide

## Project Overview
Crab is a distributed restaurant management system written in Rust, featuring an Edge Server and Client architecture. It focuses on reliability, offline capabilities, and type-safe communication.

## Architecture
- **Workspace**:
  - `shared`: Common types, protocols, and message definitions (`OrderIntent`, `OrderSync`).
  - `edge-server`: The core edge node. Handles HTTP/TCP requests, database (SurrealDB), and message broadcasting.
  - `crab-client`: Unified client library supporting both Network (HTTP/TCP) and In-Process (Oneshot/Memory) communication.

## Build & Test Commands
- **Build**: `cargo build --workspace`
- **Check**: `cargo check --workspace`
- **Test**: `cargo test --workspace`
- **Lint**: `cargo clippy --workspace -- -D warnings`
- **Format**: `cargo fmt`

## Run Examples
- **Interactive Server Demo**:
  ```bash
  cargo run -p edge-server --example interactive_demo
  ```
- **Message Client Demo**:
  ```bash
  cargo run -p crab-client --example message_client
  ```

## Key Protocols & Patterns
- **Message Bus**:
  - Uses `OrderIntent` (Client -> Server) and `OrderSync` (Server -> Client) for state changes.
  - Payloads are defined in `shared::message`.
  - Supports both TCP (network) and Memory (in-process) transports.
- **Server State**:
  - `ServerState` is initialized via `ServerState::initialize(&config).await`.
  - Do NOT use `ServerState::new(...)` directly for initialization logic; it is a pure constructor.
  - `ServerState` is designed to be clone-cheap (uses `Arc`).
- **Client**:
  - `CrabClient` trait unifies `Http` and `Oneshot` backends.
  - `MessageClient` handles real-time bidirectional communication.

## Coding Standards
- **Error Handling**: 
  - **Current Phase (PoC/Alpha)**: `unwrap()`/`expect()` are permitted for rapid prototyping and asserting invariants in controlled environments.
  - **Production Goal**: Move towards strict, typed error handling (`AppError`, `Result<T, E>`). Eliminate panics in runtime paths.
- **Async**: Prefer `tokio`. Use `#[async_trait]` for traits with async methods.
- **Ownership**: Prefer borrowing over cloning. Use `Arc` for shared state.
- **Documentation**: Document public APIs with examples. Run `cargo test --doc` to verify.

## Project Status & Philosophy
- **Phase**: **Feasibility Testing / Prototype**
- **Edge Server Focus**: 
  - Designed as an **Edge Node**: Self-contained, offline-capable, and maintenance-free.
  - **Embedded DB**: Uses embedded SurrealDB to avoid external dependencies.
  - **Future Roadmap**: Transition to strong typing enforcement and robust error handling as the project matures from prototype to production.

## User Preferences (from Custom Instructions)
- **Language**: Rust Idiomatic.
- **Concurrency**: Safe patterns (`Arc<Mutex<T>>`, channels).
- **Type System**: Leverage newtypes and traits to enforce invariants.
- **Response Language**: Chinese (Answer in Chinese).
