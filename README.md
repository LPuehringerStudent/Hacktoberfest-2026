# Phishing Dojo <!-- TODO: final name -->

> Practise spotting phishing emails and scam phone calls in a safe simulator, with realistic scenarios written by Gemma 4 and live AI callers voiced by ElevenLabs.

Built at the **MLH x YAIL Hacktoberfest HackDay** (JKU Linz, 30 Oct 2026).

## Why

Most breaches start with a person, not a bug. Awareness training is usually a slideshow. In Phishing Dojo, staff of a fictional company work through their inbox and answer their phone. Each email or call is either a normal work request or a scam. They decide whether to **do it** or **report it**, and a debrief shows exactly which red flags they caught or missed.

## Demo

- Live app: https://hacktober.lpuehringer.dev
- Video: <!-- TODO -->
<!-- TODO: screenshots (inbox, link hover, call screen, debrief) -->

## How it works

1. **Scenarios by Gemma 4.** [`scripts/generate_scenarios.py`](scripts/generate_scenarios.py) gives Gemma 4 a fact sheet for the fictional *Donauwerk Logistik GmbH* (people, internal domains, normal processes like four-eyes approval and IT never asking for passwords) and asks for one scenario at a time as JSON. It covers scam and legitimate emails and calls. Every answer is validated against the backend's schema and our content rules (only reserved `.example` domains, every scam labelled with red flags and evidence, every legit one with safe signals). Invalid answers go back to Gemma with the errors until they pass. The result is [`backend/fixtures/scenarios.json`](backend/fixtures/scenarios.json).
2. **Emails.** A fake inbox built in Lovable. Hovering a link shows where it really goes, links open in-app landing pages (fake Microsoft 365 login, intranet), and attachments open fake previews. Typing a password into a scam page or clicking *Enable content* counts as falling for it.
3. **Live calls.** An ElevenLabs voice agent calls you. Our Rust backend is the agent's *custom LLM*: for each turn it swaps in the scenario's persona (kept server-side, so the browser never sees the answer) and streams Gemma 4's reply back to ElevenLabs. You can talk, push back, or hang up and report.
4. **Debrief.** After each scenario: right or wrong, then each red flag with the exact evidence. At the end of a round of 5 mixed scenarios: a score.

```
┌───────────────────┐  /api/round, scenarios  ┌──────────────────────┐   ┌─────────────────────┐
│ Lovable frontend  │ ──────────────────────▶ │ Rust backend (Axum)  │   │ Gemma 4             │
│ inbox, call, score│ ◀────────────────────── │ strips answer fields │   │ gemma-4-26b-a4b-it  │
└─────────┬─────────┘                         │ /v1/chat/completions │──▶│ (Gemini API or      │
          │ voice session (scenario_id)       └──────────▲───────────┘   │  local llama-server)│
          ▼                                              │ custom LLM    └─────────────────────┘
┌───────────────────┐                                    │
│ ElevenLabs agent  │ ───────────────────────────────────┘
│ STT, turns, voice │
└───────────────────┘
```

More detail: [`docs/PLAN.md`](docs/PLAN.md), [`docs/API.md`](docs/API.md), [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Challenges we're entering

| Challenge | How this project qualifies |
|---|---|
| Best Open-Source AI Project (DigitalOcean) | Open-weight **Gemma 4** writes every scenario and plays every caller; without it there is no app. The repo is public and MIT-licensed. Gemma 4 26B A4B also runs on our own CPU server with llama.cpp (Q4_0, ~11.6 tokens/s writing, see [`docs/PLAN.md`](docs/PLAN.md#model-hosting)), and the backend switches to it with one env var (`MODEL_BASE_URL`). |
| Best Use of Gemma 4 | `gemma-4-26b-a4b-it` with **system instructions** (company fact sheet, security-training framing), **structured JSON output** validated against a typed Rust schema with a self-correction loop, and **real-time role-play** as phone personas streamed to a voice agent. <!-- TODO: add function calling (end_call) if it works in the demo --> |
| Best Use of ElevenLabs | Live voice calls are half of the training: an ElevenLabs agent with our backend as its custom LLM, so the scam caller (or the genuine colleague) is a real conversation you can push back on. |
| Best Use of Lovable | The whole frontend (inbox, link hover, landing pages, attachment previews, call screen, debrief) is built in Lovable: <!-- TODO: Lovable repo URL -->. Mirrored into [`frontend/`](frontend/). |

## Run it locally

```bash
cp .env.example .env                       # GEMINI_API_KEY, ELEVENLABS_API_KEY
cd backend && cargo run                    # http://127.0.0.1:8000, see backend/README.md
python3 scripts/generate_scenarios.py      # optional: regenerate the 20 scenarios with Gemma 4
```

To use a self-hosted Gemma 4 instead of the Gemini API, run `llama-server` with the GGUF weights and set `MODEL_BASE_URL=http://127.0.0.1:8080/v1`. Frontend: see [`frontend/README.md`](frontend/README.md).

## Safety

Everything is fictional: one made-up company, and only the reserved `.example` top-level domain, so no scenario points at a real or look-alike site. The backend only serves scenarios for that company. There are no user-chosen targets or free-text prompts, and the answer fields (red flags, caller goals, personas) never reach the browser.

## What we learned

<!-- TODO: one or two lines each, what was new for you -->

## Team

| Person | Role |
|---|---|
| <!-- name --> | Scenarios with Gemma 4 |
| <!-- name --> | ElevenLabs voice agent |
| Laurenz | Rust backend, deploy, call relay |
| <!-- name --> | Lovable frontend & design |

## AI tools used

MLH rules require disclosure of AI tools used while building:

- **Gemma 4** (`gemma-4-26b-a4b-it`): runtime model for scenarios and callers
- **ElevenLabs**: runtime voice agent
- **Lovable**: frontend generation
- **Claude Code** (Claude Opus 5.5): coding assistant (scenario generator, docs, backend)
- <!-- TODO: other coding assistants used -->

## License

[MIT](LICENSE)
