# 2-minute presentation

**Final pitch for the MLH x YAIL Hacktoberfest HackDay judges.** One person speaks, one drives. Target: 118 seconds, so you have slack.

**Slides:** open [`presentation/index.html`](presentation/index.html) in a browser (press `S` for speaker notes, `F` for fullscreen). The only missing piece is `presentation/demo.mp4` — the 50-second recording. This file is the offline backup if the Wi-Fi dies.

Judges score **Technology, Design, Completion, Learning**, equally weighted. They grade what they see working, not the pitch — so the video does the heavy lifting and the words only frame it.

## Run sheet

| Time | What | Spoken words |
|---|---|---|
| 0:00–0:12 | Hook + the 62% | 30 |
| 0:12–1:02 | Demo video, 50 s — **you stay silent** | — |
| 1:02–1:43 | Four challenges, one sentence each | 94 |
| 1:43–1:54 | The hard part (io_uring) | 22 |
| 1:54–2:00 | Close | 10 |

The 62% costs you two seconds, so this now lands right on 2:00. If the timer is red, drop the io_uring line, not the number.

Read this aloud once with a stopwatch before you go on. It should land at 65–70 seconds of speech. If you are over 75, cut a category sentence, not the hook.

## Spoken script

> **0:00 — Hook**
>
> "**62% of breaches** involve a human being — not a bug in the code. Slideshows don't fix that. So we built a gym for the reflex to stop and check: **Tripwire**."
>
> The 62% is **Verizon's 2026 Data Breach Investigations Report** (up from 60% the year before). It is on the slide, so a judge can check it — never quote a stat you can't source in one sentence.
>
> Alternative if you'd rather tell a story than cite a number: *"Casinos went dark for days because someone called the IT help desk and asked nicely."* That's the 2023 MGM Resorts breach — a phone call, exactly what we simulate. Verify the wording before you use it.

*Start the video on the scam email itself. No title card. The pause after "Tripwire" is the cut.*

> **0:10 — ▶ Video (50 s). Do not talk over the ElevenLabs audio.**

> **1:00 — Lovable (Design)**
>
> "Lovable built the whole frontend: an inbox anyone can use with zero training — hover a link and it shows where it really goes."

> **1:10 — Open weights / self-hosted (Open-Source AI)**
>
> "Every scenario and every caller is Gemma 4 running on our own server. You can't send your staff's training data to someone else's cloud."

> **1:21 — Gemma 4 (Best Use of Gemma)**
>
> "Gemma plays the scammer too — it ends the call with a function call once you've given it what it wants, and JSON validation means a broken scenario can never ship."

> **1:33 — ElevenLabs (Best Use of ElevenLabs)**
>
> "ElevenLabs makes it a real phone call. You can push back, ask for a callback — and it argues with you."

> **1:41 — The hard part (Technology)**
>
> "Underneath: a hand-written io_uring server, one point five to two point three times faster than our Axum baseline. Same routes, one env var."

> **1:52 — Close**
>
> "Your firewall is fine. Your people need practice. Thank you."

**Do not say "Questions?"** Just stop. The silence invites them better than asking, and it costs you nothing.

## If you are running short on time

Cut in this order, and keep the hooks:

1. Drop the io_uring sentence (1:41). Painful — it is the best Technology line — but it is the only part nobody can *see*.
2. Drop the EleventhLabs sentence (1:33) — the video already demonstrates it with audio.
3. Never cut the hook or the closer. They are the only two lines anyone will remember.

## Why each sentence is framed this way

Each one has to win a *challenge* and hit a *judging criterion* at the same time.

- **Lovable → Design.** Don't say "we used Lovable" — say what it bought you: a non-technical employee starts in one click. The hover-to-reveal on screen is the proof.
- **Open weights → Technology + architecture.** The strong argument is not "we used Gemma." It is that a *security* trainer cannot ship employees' behaviour to a third-party cloud API. Self-hosting is why the weight licence matters, not a nice-to-have.
- **Gemma 4 → Technology.** Function calling plus structured JSON validated against a Rust schema is the "beyond plain chat" requirement.
- **ElevenLabs → Technology.** Speech in, not just text-to-speech. "It argues with you" is the difference between meaningful and decorative.

## The impressive lines, ranked (only #1 fits at 2 minutes)

1. **io_uring, 1.5–2.3× Axum.** Almost nobody writes their own HTTP server at a hackathon. Best points-per-word in the project — but invisible in a demo, which is exactly why it is spoken while the video is off.
2. **You can't cheat.** Answer fields are stripped server-side at any depth; the persona never reaches the browser. Keep as a half-sentence in reserve — judges silently suspect every demo is mocked.
3. **Group-commit event log → a real training metric** ("60% of our testers clicked the fake login"). Too long for 2 minutes; use it as an *answer* to "is this a product or a demo?"

## Q&A bank

Likely questions, and the answer that is true:

- **"How is this different from KnowBe4?"** → Those send real emails with real links to real staff. Ours is one fictional company, only reserved `.example` domains, and no real site is ever touched. And we do voice — most trainers can't.
- **"Does the AI decide whether I passed?"** → No. Labels are written and validated at generation time against a Rust schema, and the browser never receives them. Open devtools — the answer isn't there.
- **"Won't people just always click Report?"** → Half the round is legitimate requests, and reporting a real one costs a point. False alarms are scored exactly like misses.
- **"What was new for you?"** → One specific thing per person. *"I'd never made a model hold a character under real-time streaming constraints"* beats *"we learned about LLMs."* Same four lines belong in the README's *What we learned*.

## Before you record (do these first)

1. **`/api/round/summary` and `/api/stats` are not wired into the frontend.** The round-end Gemma coaching is backend-only: `frontend/src/routes/index.tsx` keeps the score client-side and `frontend/src/lib/api.ts` has no `summary` call. Either wire it or leave it out of the video — do not promise a screen that is not there. Re-sync with `scripts/sync-frontend.sh` and check what Lovable has added since commit #36.
2. **One name.** The UI says **Tripwire**_; the README and `docs/PLAN.md` still say *Phishing Dojo*. Use Tripwire everywhere.
3. **Verify `end_call` actually fires** on a real call before claiming function calling. `README.md` still carries a TODO saying "if it works in the demo." If it is flaky, cut those four words.
4. **Don't script the "you noticed it but clicked anyway" debrief.** `decide()` does not send `observed`, so that state never renders.

## Delivery

- Record the video **with its own audio**. Do not narrate live — the Wi-Fi dies at the worst moment, and you need the recording to work as a backup.
- Have the MP4 on the laptop, not just in the cloud.
- One speaker. If four people take turns, every handoff eats 3 seconds you don't have.
- If a judge interrupts with a question mid-pitch, answer it in one sentence and go straight back to the next line.
