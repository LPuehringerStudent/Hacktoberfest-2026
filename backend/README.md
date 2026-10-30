# backend/

Rust (edition 2024) + Axum 0.8 + Tokio. See [`docs/PLAN.md`](../docs/PLAN.md) for the design and [`docs/API.md`](../docs/API.md) for the contract.

```bash
cargo run                      # http://127.0.0.1:8000  (env: HOST, PORT, CORS_ORIGINS, RUST_LOG)
cargo test                     # route smoke tests: must stay green
cargo fmt && cargo clippy --all-targets -- -D warnings   # CI runs both
```

| Path | What |
|---|---|
| `src/lib.rs` | `app()`: router, CORS, state |
| `src/routes.rs` | Handlers |
| `src/scenario.rs` | Scenario schema (the contract) + stripping of answer fields |
| `fixtures/scenarios.json` | Mock scenarios served until the Gemma pool exists |
| `tests/smoke.rs` | Route smoke tests. Any server implementation (Axum, io_uring) must pass them |

`main` is autodeployed to https://hacktober-api.lpuehringer.dev (see `docs/ARCHITECTURE.md`). The deploy runs `cargo build --release` and starts `target/release/backend`.

Test unmerged work with the Lovable preview through a temporary public URL:

```bash
cloudflared tunnel --url http://localhost:8000
```
