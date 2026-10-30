# Architecture

<!-- TODO at kickoff: replace with the real design once the idea is locked. -->

```
┌──────────────────────┐   HTTP/JSON (docs/API.md)   ┌─────────────────────────┐
│ Frontend (Lovable)   │ ──────────────────────────▶ │ Backend (FastAPI)       │
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
| Python + FastAPI backend | Best SDK support for google-genai and ElevenLabs. Fast to write. |

## Deployment

Domain `lpuehringer.dev` is on Cloudflare (DNS owned by Laurenz).

| URL | What | How |
|---|---|---|
| `https://hacktober.lpuehringer.dev` | Frontend (published Lovable app) | Lovable custom domain (paid plan, covered by the Pro perk). Add the DNS records Lovable shows in Cloudflare as **DNS only** (grey cloud) |
| `https://hacktober-api.lpuehringer.dev` | Backend (FastAPI), **always the latest `main`** | Autodeployed on Laurenz's server, exposed via Cloudflare tunnel `hacktober` |

- Use `hacktober-api.…`, not `api.hacktober.…`. Cloudflare's free certificate only covers one subdomain level.

### Autodeploy

The server polls GitHub every 30 s. When `main` moves, it runs `git reset --hard origin/main`, `uv sync` in `backend/`, restarts the backend, and checks `GET /health`. So **merging a PR deploys it** within about a minute. A broken `main` means a broken demo backend, so CI and review matter.

- The backend needs `backend/pyproject.toml` and `app.main:app` with a `/health` route. Until those exist, the URL returns HTTP 502/530.
- Secrets live on the server in `/etc/hacktober/backend.env` (same keys as `.env.example`). Ask Laurenz to update them.
- Logs: `ssh hack journalctl -u hacktober-backend -u hacktober-deploy -f` (Laurenz).

### Testing unmerged backend work with Lovable

The named tunnel belongs to the server. **Don't run it from your laptop.** Use a throwaway quick tunnel instead and point Lovable's preview at the random URL temporarily:

```bash
cloudflared tunnel --url http://localhost:8000   # prints https://<random>.trycloudflare.com
```
