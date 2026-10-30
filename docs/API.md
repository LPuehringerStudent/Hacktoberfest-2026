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

### `POST /api/round` (mock)

Starts a round. Request body: none.

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

### `POST /api/scenario/{id}/event` (mock: logged, not stored yet) → `204`

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

### Planned

| Endpoint | Purpose |
|---|---|
| `GET /api/call/{id}/debrief` | Debrief after a call (outcome from Gemma's tool calls + transcript), same shape as the decision response |
| `POST /v1/chat/completions` | ElevenLabs custom-LLM endpoint (OpenAI-compatible, SSE). Not called by the frontend |
| `POST /api/check-email` | Multipart screenshot → `{ verdict, redFlags: [{ type, evidence }] }` |
| `GET /api/stats` | Aggregates from the event log |
