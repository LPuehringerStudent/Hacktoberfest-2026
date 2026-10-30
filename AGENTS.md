# AGENTS.md: rules for AI coding agents

Applies to every agent working in this repo (Claude Code, Codex, Cursor, Copilot, Gemini CLI, …). Humans: see [CONTRIBUTING.md](CONTRIBUTING.md). These rules restate it for agents and add a few.

## Context

- 4-person team at a ~1-day hackathon (MLH x YAIL Hacktoberfest HackDay). Speed matters more than polish, but `main` must always run.
- Goal: qualify for **all four** challenges. Read [`docs/CHALLENGES.md`](docs/CHALLENGES.md) before adding features, and don't remove anything that a challenge depends on.
- Judges grade **Technology, Design, Completion, Learning**. They don't grade code quality.

## Hard rules

1. **Never push to `main`.** Work on a branch named `claude/<topic>` (or `<agent>/<topic>`) and open a PR.
2. **Never edit `frontend/`.** It's a git-subtree mirror of the Lovable repo. Frontend changes belong in the Lovable repo.
3. **Never commit secrets.** Keys come from env vars (`.env`, git-ignored). Add new variable names to `.env.example` with an empty value.
4. **Don't change the API contract silently.** If an endpoint's shape changes, update [`docs/API.md`](docs/API.md) in the same PR and call it out in the PR description.
5. **Gemma 4 must stay an essential component.** It's required for the Open-Source AI challenge. Don't swap it for a closed model (e.g. Gemini) in any core path.
6. **Disclose yourself.** Add a `Co-Authored-By:` trailer (or your tool's equivalent) to commits, and fill in the PR template's *AI tools used* line.
7. Don't add new runtime dependencies or services casually. Say why in the PR.
8. Keep PRs small and focused. One feature or fix per PR.

## Tech defaults (until the team decides otherwise in `docs/ARCHITECTURE.md`)

- **Backend:** Python 3.12 + FastAPI in `backend/`. Use `uv` for deps.
- **Gemma 4:** `google-genai` SDK, env `GEMINI_API_KEY`, model `gemma-4-26b-a4b-it` (fast MoE) or `gemma-4-31b-it` (stronger). Supports image input, function calling, structured JSON output, and system instructions.
- **ElevenLabs:** official `elevenlabs` Python SDK, env `ELEVENLABS_API_KEY`. Called from the backend only.
- **Frontend:** whatever Lovable generates (React + Vite + TS + Tailwind). It calls the backend at a public URL set in `src/config.ts` (Lovable's preview can't reach `localhost`). The backend must allow CORS from `*.lovable.app` / `*.lovableproject.com`.
- SDKs move fast. **Look up current docs** (context7 / official docs) instead of relying on memory for model IDs and SDK signatures.

## Before opening a PR

- It runs locally (start the backend and hit the endpoint you touched).
- No keys in the diff (`git diff --cached | grep -iE 'key|token|secret'`).
- `docs/API.md` is updated if the contract changed.
- The PR template is filled in, including the AI tools line.

## When unsure

Ask the human driving you instead of guessing. Product decisions (what the app does, how it looks) belong to the team, not the agent.
