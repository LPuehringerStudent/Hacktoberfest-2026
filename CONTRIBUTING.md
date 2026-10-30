# Contributing: team workflow

This is for all 4 of us **and** for any AI agent we run. Agents should also read [AGENTS.md](AGENTS.md).

## TL;DR

1. Pick or create a GitHub Issue and assign yourself.
2. `git switch main && git pull && git switch -c <you>/<topic>`
3. Commit small, push often, and open a **draft PR** early.
4. Mark it ready, get one 👍 from anyone, then **squash-merge**. Delete the branch.
5. Pull `main` often. Merge `main` into your branch rather than force-pushing.

## Roles

| Role | Owns | Folder |
|---|---|---|
| **Dev 1: AI** | Gemma 4 prompts, multimodal input, function calling, model logic | `backend/` (ai module) |
| **Dev 2: Voice + API** | ElevenLabs (TTS/STT), HTTP API that the frontend calls | `backend/` (api + voice modules) |
| **Dev 3: Integration** | Repo, deploy, env/secrets, frontend sync, glue, demo video, submission | `scripts/`, `.github/`, `docs/` |
| **Lovable** | UI/UX in Lovable, the Lovable repo, design | Lovable repo → mirrored to `frontend/` |

Owners decide quickly in their area. Cross-cutting decisions get settled in 2 minutes out loud, and then **written into `docs/`**.

## Repo layout

```
backend/        Python API: Gemma 4 + ElevenLabs. Devs work here.
frontend/       READ-ONLY mirror of the Lovable repo (git subtree). Don't edit here.
docs/           API contract, architecture, challenge checklist, submission plan
scripts/        Helper scripts (frontend sync, ...)
.github/        PR/issue templates, CI
```

## Lovable ⇄ this repo

Lovable can only sync to a repo it creates itself, with the app at the repo root. So:

- The **Lovable repo** (separate, public, owned by the Lovable person) is the source of truth for the frontend.
- `frontend/` in this repo is a mirror, updated with `scripts/sync-frontend.sh` by Dev 3 or whoever needs it.
- **Never edit `frontend/` here.** Those changes would be lost or cause subtree conflicts. Frontend changes go into Lovable, or as a push to the Lovable repo's `main` (it syncs back into Lovable).
- The frontend talks to the backend **only** through the HTTP API described in [`docs/API.md`](docs/API.md). API keys **never** go in the frontend.

### How Lovable learns about the backend

Lovable can't see this repo. Its AI only knows what we tell it:

1. **Project knowledge:** paste `docs/API.md` into the Lovable project's knowledge, and re-paste it whenever it changes (devs: ping the Lovable person). FastAPI also serves the live schema at `<backend-url>/openapi.json`, which can be pasted too.
2. **Backend URL:** in one place in the frontend: `src/config.ts` → `export const API_URL = "https://hacktober-api.lpuehringer.dev"`. Tell Lovable to use it for every call.
3. **Reachable backend:** Lovable's preview runs in the cloud, so it **can't reach `localhost`**. The backend is served through a Cloudflare tunnel at `hacktober-api.lpuehringer.dev` (setup in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md#deployment)). Get this working within the first hour.
4. **CORS:** the backend must allow `https://hacktober.lpuehringer.dev` and the Lovable preview origins (`*.lovable.app`, `*.lovableproject.com`). Check the actual origin in the browser devtools if requests fail.

## Contract-first

`docs/API.md` is the handshake between the Lovable person and the devs. Change it **before** you change an endpoint, in its own small PR, and tell the team. Until an endpoint is real, the backend can return mocked data in the agreed shape so the frontend is never blocked.

## Branches & commits

- Branch names: `<name>/<topic>`, e.g. `anna/gemma-vision`, `claude/tts-endpoint` for agent-driven branches.
- Commit messages: `type(scope): summary`. Types: `feat`, `fix`, `docs`, `chore`, `refactor`. Scopes: `ai`, `voice`, `api`, `frontend`, `infra`, `docs`.
- Nobody pushes directly to `main` (people or agents). Everything goes through a PR.
- Squash-merge only, so `main` stays a readable log of features for the judges.

## Pull requests

- Open a draft PR as soon as you push. It shows everyone what you're touching.
- Fill in the template. The **AI tools used** line is mandatory (MLH rule: we must be transparent about AI tools).
- Review is a quick sanity check: does it run, does it break the API contract, does it leak a key? Aim for < 10 minutes. Don't block on style. Judges don't grade code quality.
- Merging is the author's job once it has 1 approval and CI is green.

## Secrets

- Keys go in `.env` (git-ignored). Add new variable *names* to `.env.example`.
- A leaked key in git history must be **rotated immediately**. Deleting the commit isn't enough because the repo is public.
- CI runs gitleaks on every PR.

## Rules that can get us disqualified (MLH)

- **All project work happens during the hackathon.** No code from before the event. Libraries and frameworks are fine.
- The repo **must stay public** after the event to remain eligible for prizes.
- We **must stop hacking when time is up**. After that, only tiny bug fixes for the demo, no features.
- Be honest about AI tools (see README → *AI tools used*).
- Follow the [MLH Code of Conduct](https://mlh.io/code-of-conduct).

## Timeline (fill in times at the opening ceremony)

| When | What |
|---|---|
| Kickoff | Lock the idea, fill in README pitch + `docs/API.md` v1, create issues, Lovable person creates Lovable project + repo |
| +1h | Backend on a public URL (deploy or tunnel). Walking skeleton: Lovable preview → backend → Gemma → response on screen (mocked voice OK) |
| Midpoint | Every challenge feature works end-to-end at least once (see `docs/CHALLENGES.md`) |
| **Deadline − 2h** | **Feature freeze.** Only bug fixes, polish, demo prep |
| Deadline − 1h | Record demo video, finish README, submit on OrganizerHQ |
| Deadline | Stop. |
