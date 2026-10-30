# Plan: phishing & social-engineering trainer

Working name: **Phishing Dojo** <!-- TODO: final name -->

## Pitch

Most breaches start with a person, not a bug. Non-technical staff practise spotting scams in a safe simulator. They work through emails (with links and attachments) and **live phone calls** with an AI caller. Each one is either a normal work request or a scam. They either do what's asked or report it, and a debrief shows which red flags they caught or missed.

## Scope for the 18:30 submission (cut at 15:45)

**This section overrides the rest of the document.** Anything not listed here is cut.

| Time | Milestone |
|---|---|
| **16:45** | Each part works on its own: frontend on the mock API, ~20 Gemma scenarios committed, a live ElevenLabs call with a Gemma persona |
| **17:15** | Integrated: a full round of emails + calls → debrief → score on the published Lovable app |
| **17:30** | **Feature freeze.** Only bug fixes after this |
| 17:30–18:00 | README (pitch, screenshots, AI tools used, what we learned), demo video with audio |
| **18:00–18:20** | Submit on OrganizerHQ to **all 4 challenges**. 10 min buffer |

| Who | Builds | Card |
|---|---|---|
| **Lovable** | Inbox + email view + Do it / Report, link hover + `m365_login` / `intranet_page` landing pages, `pdf` / `docx_macro` attachment previews, call screen, debrief + score | #4 #5 #16 #17 #18 #19 |
| **Dev 2** | Script that asks Gemma 4 (Gemini API) for ~20 scenarios, validates them against the schema, and **commits them to `backend/fixtures/scenarios.json`**. No live generation | #6 |
| **Dev 3** | ElevenLabs agent: custom LLM = Gemma 4, persona per scenario, `end_call`, voices. Test against Gemini's OpenAI-compatible endpoint directly first, then switch to Laurenz's relay | #8 |
| **Laurenz** | Round = 5 random scenarios, mixed scam/legit. `POST /v1/chat/completions` relay: loads the persona for `scenario_id` server-side, streams from Gemma 4 | #9 #12 |
| **Everyone at 17:30** | README, video, submission | #20 |

How the call ends (simplified): **Hang up & report** → decision `report`. The agent ends the call (`end_call`) after getting what it wanted or finishing the legit request → decision `comply`. The debrief uses the scenario's `redFlags`. No per-turn tool tracking.

**Cut:** io_uring server, SQLite + group commit, live scenario pool, local llama-server / MTP, rate limit, Gemma-written debriefs, stats page, check-a-real-email, German, `SEND_ZC`. The **Open-Source AI** entry still qualifies: Gemma 4 is open-weight and essential (scenarios + caller), the repo is public and MIT-licensed. Mention in the README that the benchmark shows it runs on our own CPU server.

## Decisions

"Team" = decided in the grilling session. "Claude's call" = a reasonable default, so object if you disagree.

| # | Decision | By |
|---|---|---|
| 1 | One fictional company, **Donauwerk Logistik GmbH** (Linz, ~400 staff), given to Gemma with every scenario. Domain `donauwerk.example`. All domains use the reserved `.example` TLD, so no real or look-alike domains | Team / name: Claude's call |
| 2 | English only. German is a stretch goal | Team |
| 3 | A round = 5 mixed scenarios (emails + calls, ~50% scams) → final score | Team |
| 4 | Emails have links (open a fake page in the app) and attachments (fake previews). No real files, no real URLs | Team |
| 5 | Calls are live voice conversations. User can **Hang up & report** at any time. Gemma ends the call via a tool call when the user gives away the target (fell for it) or when a legit request is done | Team |
| 6 | Fixed red-flag list (below). Every scenario is labelled. Legit scenarios list the flags they *don't* have | Team |
| 7 | Same model for legit and scam scenarios | Team |
| 8 | Rust backend: io_uring, multishot accept/recv, grouped SQLite transactions, + extras (#14) | Team |
| 9 | No accounts. Score kept in browser `localStorage` | Team |
| 10 | Backend only ever generates scenarios for the fictional company. No user-chosen targets or free-text scenarios. Per-IP rate limit | Team |
| 11 | SQLite holds only: scenario cache + anonymous event log | Team |
| 12 | Inbound server built directly on the `io-uring` crate, with an Axum fallback behind a cargo feature | Team |
| 13 | **Build routes on Axum first**, swap in the io_uring server once it passes the same smoke test. The frontend is never blocked by the risky part | Claude's call |
| 14 | Extras: JSON-schema/grammar-constrained output, MTP speculative decoding, llama-server prompt cache. `SEND_ZC` is a stretch goal | Team |
| 15 | "Check a real email": upload a screenshot → Gemma 4 image input → red flags. Built last | Team |
| 16 | Autodeploy switches to Rust (`cargo build --release` on the server). CI gets `cargo check`. Docs are updated from Python to Rust | Team |
| 17 | Model hosting: see [Model hosting](#model-hosting), pending benchmark | Pending |

## Red flags

| id | Meaning |
|---|---|
| `urgency` | Artificial time pressure ("within 30 minutes or your account is locked") |
| `authority` | Claims to be the CEO, IT, police, bank… to shortcut thinking |
| `credential_request` | Asks for a password, MFA code, or login via a link |
| `payment_change` | New bank details, gift cards, unusual transfer |
| `sender_mismatch` | Display name or caller ID doesn't match the real address/number |
| `link_mismatch` | Link text ≠ real destination, or a look-alike domain |
| `risky_attachment` | Macros, double extensions (`.pdf.exe`), `.html`/`.zip` with a password |
| `secrecy` | "Don't tell anyone / don't call me back" |
| `process_bypass` | Asks to skip the normal procedure (ticket, four-eyes approval, callback) |

## User flow

1. **Start round** → 5 scenario cards (email or call).
2. **Email:** a fake inbox view. Hovering a link shows the real target. Clicking opens an in-app landing page (fake M365 login, intranet page, invoice portal…). Typing credentials into a scam page = **fell for it**. Attachment chips open a fake preview. Clicking "Enable content" on a scam doc = **fell for it**. Buttons: **Do it** / **Report as phishing**.
3. **Call:** phone screen, the voice caller speaks first, and the user talks back. Ends by **Hang up & report**, by Gemma's `user_complied` tool (fell for it), or by `request_complete` (legit, done).
4. **Debrief** after each scenario: correct or not, then each red flag with the exact evidence (highlighted email text or transcript line), plus what they should have done.
5. **Round score** + "what to watch for" summary. A stats page shows aggregate numbers from the event log.
6. Separate tab: **Check a real email** (screenshot upload).

## Architecture

```
Lovable app (hacktober.lpuehringer.dev)
  │  REST/JSON                         │ WebRTC/WS voice (@elevenlabs/react)
  ▼                                    ▼
Rust backend  ◀── /v1/chat/completions ── ElevenLabs Agent (custom LLM)
(hacktober-api.lpuehringer.dev)
  ├─ inbound:  io_uring server (Axum fallback)
  ├─ handlers: Tokio runtime
  ├─ model client (OpenAI-compatible) ──▶ llama-server on R740 (Gemma 4 26B A4B)  and/or  Gemini API (gemma-4-26b-a4b-it)
  └─ SQLite (WAL): scenarios, events  ← single writer thread, group commit
```

### Scenario schema (the contract: everyone builds against this)

```jsonc
{
  "id": "sc_8f2a",
  "channel": "email",            // "email" | "call"
  "isScam": true,                 // hidden from the client until the debrief
  "redFlags": [{ "type": "link_mismatch", "evidence": "href → m365-donauwerk.example" }],
  "safeSignals": [],              // legit scenarios: why it's fine
  "correctAction": "report",      // "comply" | "report"
  "email": {
    "fromName": "IT Service Desk", "fromAddress": "it-desk@donauwerk-support.example",
    "subject": "…", "bodyMarkdown": "… [Verify account](link:l1) …",
    "links": [{ "id": "l1", "shownHref": "https://login.donauwerk.example", "realHref": "https://m365-donauwerk.example/login",
               "landing": { "template": "m365_login", "brand": "Microsoft 365", "trap": true } }],
    "attachments": [{ "id": "a1", "filename": "Rechnung_4471.docm", "kind": "docx_macro",
                      "preview": { "title": "…", "text": "…" }, "trap": true }]
  },
  "call": {
    "callerName": "Markus from IT", "callerId": "+43 732 000 000",
    "goal": "get the user's MFA code", "target": "mfa_code",
    "openingLine": "…", "voiceId": "…", "personaPrompt": "…"
  }
}
```

Landing templates: `m365_login`, `intranet_page`, `invoice_portal`, `parcel_tracking`. Attachment kinds: `pdf`, `docx_macro`, `double_extension`, `html_login`, `zip_password`. Lovable builds one component per template/kind.

### API (moves into `docs/API.md` once approved)

| Method & path | Purpose |
|---|---|
| `POST /api/round` | → `{ roundId, scenarios: [{ id, channel }] }` (5, from the cache pool) |
| `GET /api/scenario/:id` | Scenario **without** `isScam`, `redFlags`, `safeSignals`, `correctAction`, `trap` |
| `POST /api/scenario/:id/event` | `{ type: "open" \| "hover_link" \| "click_link" \| "open_attachment" \| "submit_credentials" \| "enable_content", detail }` → 204 |
| `POST /api/scenario/:id/decision` | `{ action: "comply" \| "report" \| "fell_for_it" }` → debrief `{ correct, redFlags, caught, missed, explanation }` |
| `GET /api/call/:id/debrief` | Debrief after a call (outcome recorded from the tool calls + transcript) |
| `POST /v1/chat/completions` | ElevenLabs custom-LLM endpoint (OpenAI-compatible, SSE streaming) |
| `POST /api/check-email` | Multipart screenshot → `{ verdict, redFlags: [{ type, evidence }] }` |
| `GET /api/stats` | Aggregates from the event log |
| `GET /health` | Autodeploy health check |

### Live call

- **ElevenLabs Agent** with **Custom LLM** pointing at `https://hacktober-api.lpuehringer.dev/v1`. ElevenLabs handles the microphone, speech-to-text, turn-taking and the voice. Our backend supplies the words.
- The frontend starts the session with dynamic variable `scenario_id`. The agent's system prompt contains `{{scenario_id}}`, so the backend knows which persona to load.
- Backend → Gemma: persona prompt + transcript. Gemma has two tools, `user_complied` and `request_complete`. When either fires, the backend records the outcome and streams back ElevenLabs' `end_call` tool.
- Replies stay short (1–2 sentences). Stream token by token so the voice starts early.
- **Verify in the first hour** (Dev 3): the custom-LLM request format, how dynamic variables arrive, and how `end_call` is triggered from a custom LLM.

### Rust backend

```
backend/
  Cargo.toml          features: default = ["uring"], "axum"
  src/main.rs
  src/uring/          thread-per-core (SO_REUSEPORT), multishot accept, multishot recv with provided
                      buffer ring, httparse; requests handed to Tokio via channel, responses/SSE chunks
                      come back and wake the ring via a registered eventfd
  src/axum_server.rs  fallback, same handlers
  src/handlers/       one file per route group
  src/model.rs        OpenAI-compatible client (llama-server or Gemini), JSON-schema-constrained output
  src/scenarios.rs    generation, validation against the schema, pool refill
  src/db.rs           rusqlite, WAL, one writer thread: batches events and commits every 50 ms or 256 events
```

- Scenarios are **pre-generated into a pool** (background task keeps ≥ 20 per channel), so starting a round is instant even with a slow model.
- Rate limit: token bucket per IP (`CF-Connecting-IP` header).

## Model hosting

Benchmark of Gemma 4 26B A4B Q4_0 on the R740 CPU (llama.cpp, one NUMA node), **with Tarock training running**:

| Threads | Prompt reading | Writing |
|---|---|---|
| 16 | 60 tok/s | **11.6 tok/s** (~8 words/s) |
| 32 | 77 tok/s | 6.1 tok/s (contention with Tarock) |

A 30-token call reply takes ~2.6 s to write. An uncached 1,500-token persona prompt would take ~25 s to read, so **the prompt cache is mandatory** for local calls (only the new turn gets read). Pausing Tarock for the demo should help. Not measured yet.

Default until the numbers arrive:
- **Scenario pool generation:** local llama-server on the R740. It's offline and batched, so speed doesn't matter, and it's the open-source story.
- **Live calls:** the faster of local and Gemini API (`gemma-4-26b-a4b-it`). Same model either way. Switching is one env var (`MODEL_BASE_URL`).
- **Abliterated Gemma:** only if plain Gemma refuses to write scenarios with the training framing. Refusal check pending.

## Risks

| Risk | Mitigation |
|---|---|
| Gemma refuses to write scams | Training framing in the system prompt; fallback abliterated Gemma 4 locally |
| Local model too slow for calls | Calls on the Gemini API, same model |
| Custom-LLM call flow differs from our assumptions | Dev 3 verifies in hour 1; fallback: ElevenLabs' built-in LLM for the call, with Gemma for scenarios + debrief only (weaker for the Gemma prize) |
| Gemini's OpenAI endpoint doesn't serve Gemma 4 for ElevenLabs | Dev 3 points the agent at Laurenz's relay, which calls Gemma through the native Gemini API |
| Running late | At 17:30 whatever works gets demoed. Emails alone + one call is still a full entry for all 4 challenges |
| Wi-Fi / API down during the demo | Recorded demo video as backup |
