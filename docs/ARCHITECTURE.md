# Architecture

<!-- TODO at kickoff: replace with the real design once the idea is locked. -->

```
┌──────────────────────┐   HTTP/JSON (docs/API.md)   ┌─────────────────────────┐
│ Frontend (Lovable)   │ ──────────────────────────▶ │ Backend (Rust, Axum)    │
│ React + Vite + TS    │ ◀────────────────────────── │                         │
│ src/config.ts API_URL│       JSON / audio          │  ai/    → Gemma 4       │──▶ Gemini API (gemma-4-*)
└──────────────────────┘                             │  voice/ → ElevenLabs    │──▶ ElevenLabs API
                                                     └─────────────────────────┘
```

- **All third-party API keys live in the backend.** The frontend only knows the public backend URL.
- The backend must be on a **public URL** from early on (Lovable's cloud preview can't reach `localhost`) and must allow CORS from `*.lovable.app` / `*.lovableproject.com`.
- **Repos:** this repo (submission, backend, docs) + the Lovable repo (frontend). The Lovable repo is mirrored into `frontend/` via `scripts/sync-frontend.sh`.

## Decisions log

Write one line per decision so nobody (human or agent) re-litigates it.

| Decision | Why |
|---|---|
| Gemma 4 via Gemini API (not self-hosted) | No GPU setup on event day. Model is still open-weight, so it counts for the Open-Source AI challenge. |
| Frontend in a separate Lovable repo, mirrored here | Lovable can only sync to a repo it creates, with the app at its root. |
| Rust backend (Axum first, io_uring server later) | Learning + "wow" for judges. Gemma and ElevenLabs are called over plain HTTP, so no SDKs are needed. See [PLAN.md](PLAN.md) |

## Deployment

Domain `lpuehringer.dev` is on Cloudflare (DNS owned by Laurenz).

| URL | What | How |
|---|---|---|
| `https://hacktober.lpuehringer.dev` | Frontend (published Lovable app) | Lovable custom domain (paid plan, covered by the Pro perk). Add the DNS records Lovable shows in Cloudflare as **DNS only** (grey cloud) |
| `https://hacktober-api.lpuehringer.dev` | Backend (Rust), **always the latest `main`** | Autodeployed on Laurenz's server, exposed via Cloudflare tunnel `hacktober` |

- Use `hacktober-api.…`, not `api.hacktober.…`. Cloudflare's free certificate only covers one subdomain level.

### Gemma 4 (self-hosted)

`hacktober-gemma` service on the same server: llama-server with Gemma 4 26B A4B Q4_0 on CPU, `127.0.0.1:8080`, 2 parallel slots. The backend uses it via `MODEL_BASE_URL=http://127.0.0.1:8080/v1`. No cloud AI is needed. The model file lives in `/dev/shm` (RAM), so after a server reboot it must be downloaded again (see `docs/PLAN.md`). For the demo, pause Tarock training (`ssh r740 'tarock-pause pause hackathon'`) for snappier replies.

### Autodeploy

The server polls GitHub every 30 s. When `main` moves, it runs `git reset --hard origin/main`, `cargo build --release` in `backend/`, restarts the backend, and checks `GET /health`. So **merging a PR deploys it** within about a minute. A broken `main` means a broken demo backend, so CI and review matter.

- The backend binary is `backend/target/release/backend` and must answer `GET /health`. Config comes from env: `HOST`, `PORT`, `CORS_ORIGINS`.
- Secrets live on the server in `/etc/hacktober/backend.env` (same keys as `.env.example`). Ask Laurenz to update them.
- Logs: `ssh hack journalctl -u hacktober-backend -u hacktober-deploy -f` (Laurenz).

### Testing unmerged backend work with Lovable

The named tunnel belongs to the server. **Don't run it from your laptop.** Use a throwaway quick tunnel instead and point Lovable's preview at the random URL temporarily:

```bash
cloudflared tunnel --url http://localhost:8000   # prints https://<random>.trycloudflare.com
```
