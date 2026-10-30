# Submission plan

Recording guide: [two-minute demo runbook and verified voice handoff](DEMO.md). Its pending checks do not imply a recording or submission is complete.

Owner: **Dev 3**, but anyone can tick boxes. Submissions go through **OrganizerHQ Challenges** and must link a **public, open-source GitHub repo**.

## Before the deadline

- [ ] Every box in [`CHALLENGES.md`](CHALLENGES.md) is ✅.
- [ ] Final frontend sync: `scripts/sync-frontend.sh` → PR → merged.
- [ ] README complete: pitch, demo link, screenshots, how it works, run instructions, team, **AI tools used**, **What we learned**, license.
- [ ] `docs/ARCHITECTURE.md` reflects reality.
- [ ] Deployed app reachable (backend + published Lovable URL).
- [ ] Demo video recorded (≤ 2 min is a safe target). Start with *"Hi, this is our demo for the MLH x YAIL Hacktoberfest HackDay"*. Show the Gemma 4 feature, the ElevenLabs voice (with audio), and the Lovable UI. Upload as public/unlisted.
- [ ] Submitted on OrganizerHQ to **every** challenge we qualify for. Each one may be a separate selection.
- [ ] Repo and Lovable repo are **public**. Leave them public after the event or we lose prize eligibility.

## Live demo (in person)

Judges grade what we **built**, not the pitch. Show the thing working.

1. Problem in one sentence (10s).
2. Live flow that hits Gemma 4 → ElevenLabs → Lovable UI (60–90s).
3. "The hard part was …" (Technology) + "New for us was …" (Learning) (20s).
4. Have the recorded video ready as a backup if Wi-Fi or an API dies.

## After time is up

Only tiny bug fixes that unblock the demo. No features (MLH rule 11).
