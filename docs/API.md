# API contract

This is the source of truth between the Lovable frontend and the backend. **Change this file first**, in its own PR, and tell the team.

The Lovable person can paste this file into Lovable's project knowledge so Lovable's AI generates matching calls.

## Conventions

- Base URL: the public backend URL in the frontend's `src/config.ts` (deployed or tunnelled; Lovable's preview can't reach `localhost`). Current URL: **`https://hacktober-api.lpuehringer.dev`**
- JSON in/out, `camelCase` fields.
- Errors: HTTP 4xx/5xx with body `{ "error": "human readable message" }`.
- Audio: returned as `audio/mpeg` bytes or as a URL in JSON (decide per endpoint).
- Endpoints that aren't real yet return **mock data in the final shape**, marked `(mock)` below.

## Endpoints

Status: **live** = real, **mock** = real shape with fixture data (`backend/fixtures/scenarios.json`), **planned** = not built yet.

### `GET /health` (live)

```json
{ "ok": true }
```

### `POST /api/round` (live)

Starts a round: 5 random scenarios (fewer if the pool is smaller), at least one call, 2–3 scams. Request body: none.

```json
{ "roundId": "rd_0", "scenarios": [{ "id": "fx_email_m365", "channel": "email" }, { "id": "fx_call_mfa", "channel": "call" }] }
```

### `GET /api/scenario/{id}` (mock)

The scenario **without the answers**. These fields are never sent: `isScam`, `redFlags`, `safeSignals`, `correctAction`, `trap`, `goal`, `target`, `personaPrompt`.

Email:

```json
{
  "id": "fx_email_m365",
  "channel": "email",
  "email": {
    "fromName": "Donauwerk IT Service Desk",
    "fromAddress": "it-desk@donauwerk-support.example",
    "subject": "Action required: mailbox storage limit exceeded",
    "bodyMarkdown": "… please [verify your account](link:l1) …",
    "links": [{ "id": "l1", "shownHref": "https://login.donauwerk.example", "realHref": "https://m365-donauwerk.example/login",
               "landing": { "template": "m365_login", "brand": "Microsoft 365" } }],
    "attachments": [{ "id": "a1", "filename": "Mailbox_Policy.docm", "kind": "docx_macro",
                      "preview": { "title": "Mailbox Policy 2026", "text": "… Click \"Enable content\" to view." } }]
  }
}
```

- Links in `bodyMarkdown` are written `[text](link:<id>)`. Render them as links showing `shownHref`. Hover/status bar shows `realHref`. Click opens the in-app landing page for `landing.template`.
- `landing.template`: `m365_login` | `intranet_page` | `invoice_portal` | `parcel_tracking`.
- `attachments[].kind`: `pdf` | `docx_macro` | `double_extension` | `html_login` | `zip_password`. Opening shows `preview`. Nothing is ever downloaded.

Call:

```json
{ "id": "fx_call_mfa", "channel": "call",
  "call": { "callerName": "Markus, IT Security", "callerId": "+43 732 000 1234", "openingLine": "Hi, Markus from IT security here. …", "voiceId": "…" } }
```

### `POST /api/scenario/{id}/event` (live: stored in SQLite) → `204`

```json
{ "type": "hover_link", "detail": { "linkId": "l1" } }
```

`type`: `open` | `hover_link` | `click_link` | `open_attachment` | `submit_credentials` | `enable_content`.

### `POST /api/scenario/{id}/decision` (mock: simple rules, no AI text yet)

```json
{ "action": "report" }
```

`action`: `comply` (Do it) | `report` | `fell_for_it` (typed credentials on a trap page, clicked "Enable content", …).

```json
{
  "correct": true,
  "isScam": true,
  "correctAction": "report",
  "redFlags": [{ "type": "urgency", "evidence": "…" }],
  "caught": [{ "type": "urgency", "evidence": "…" }],
  "missed": [],
  "safeSignals": [],
  "explanation": "Well spotted: this was a scam."
}
```

Red flag `type`: `urgency` | `authority` | `credential_request` | `payment_change` | `sender_mismatch` | `link_mismatch` | `risky_attachment` | `secrecy` | `process_bypass`.

### `POST /v1/chat/completions` (live): ElevenLabs custom LLM

**Not called by the frontend.** ElevenLabs calls it during a live call. OpenAI chat-completions format in, SSE stream out (passed through from Gemma 4 unchanged, including tool calls like `end_call`).

What the relay does:
1. Checks `Authorization: Bearer <RELAY_TOKEN>` (if `RELAY_TOKEN` is set on the server).
2. Finds the scenario id: `elevenlabs_extra_body.scenario_id`, or else `scenario_id: <id>` anywhere in the agent's system prompt.
3. **Replaces** the agent's system prompt with the scenario's persona + call rules (short spoken turns, stay in character, call `end_call` once the goal is reached, the request is done, or the user refuses twice). The persona never reaches the browser.
4. Forwards to Gemma 4 (`MODEL_BASE_URL`, `GEMMA_MODEL`, `GEMINI_API_KEY`) and streams the reply back.

Errors: `401` bad token, `400` no scenario id / not a call scenario, `404` unknown scenario, `502` model error.

**ElevenLabs agent setup (Dev 3):**

| Setting | Value |
|---|---|
| LLM | Custom LLM |
| Server URL | `https://hacktober-api.lpuehringer.dev/v1` (`/chat/completions` without `/v1` also works) |
| Model ID | anything (the server sets the model) |
| API key secret | the server's `RELAY_TOKEN` |
| System prompt | `scenario_id: {{scenario_id}}` (replaced server-side anyway) |
| First message | `{{opening_line}}` |
| Tools | system tool **End call** enabled |

**Frontend (Lovable), starting a call:**

```ts
conversation.startSession({
  agentId: "<from Dev 3>",
  dynamicVariables: { scenario_id: scenario.id, opening_line: scenario.call.openingLine },
});
```

When the call ends: **Hang up & report** → `POST /api/scenario/{id}/decision` `{ "action": "report" }`. Ended by the agent → `{ "action": "comply" }`. Show the returned debrief.

### `GET /api/stats` (live)

Aggregates from the event log, for a live stats panel ("60% clicked the fake login link"). `storage` shows the group commit at work: events per SQLite transaction.

```json
{
  "totals": { "decisions": 12, "correct": 9, "correctRate": 0.75, "fellForIt": 2 },
  "storage": { "events": 64, "commits": 9, "eventsPerCommit": 7.1 },
  "scenarios": [{ "id": "fx_email_m365", "channel": "email", "isScam": true, "title": "Action required: …",
                  "opens": 5, "hoveredLink": 3, "clickedLink": 2, "decisions": 5, "correct": 3, "fellForIt": 2 }]
}
```

### Planned

| Endpoint | Purpose |
|---|---|
| `GET /api/call/{id}/debrief` | Debrief after a call (outcome from Gemma's tool calls + transcript), same shape as the decision response |
| `POST /api/check-email` | Multipart screenshot → `{ verdict, redFlags: [{ type, evidence }] }` |
