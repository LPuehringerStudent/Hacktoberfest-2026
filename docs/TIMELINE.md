# Provenance: what was built when

Judges look at commit history to check nothing was built before the event. This repo was created a couple of hours early, so this page lists **everything** that existed then and everything that came after.

**The short version:** the pre-event commit is repository scaffolding — CI, PR templates, agent rules, the licence and empty documentation skeletons. It contains no application code at all: not one `.rs`, `.py`, `.ts`, `.tsx` or `.json` file. The first line of the trainer itself was committed at **15:34**.

## Timeline (30 Oct 2026, local time)

| Time | Commit | What it added | Product code |
|---|---|---|---|
| 11:39 | `6492867` | Repo setup: CI workflow, PR + issue templates, `.gitignore`, `.env.example`, `AGENTS.md`, `CONTRIBUTING.md`, `LICENSE`, README skeleton, `docs/` skeletons, `scripts/sync-frontend.sh` — 17 files | **0 lines** |
| 12:02 | `c8b6f84` | Deploy + tunnel documentation | 0 lines |
| *14:30* | — | *Event kickoff, team formation, planning. No commits.* | — |
| 15:26 | `a7a5947` | Project plan: scope, decisions, red-flag taxonomy | 0 lines |
| **15:34** | `22977d6` | **First application code**: Rust backend skeleton, scenario schema, mock fixtures — 9 files, +1,310 | **+1,310** |
| 16:07 | `fa0d1a0` | Random mixed rounds + ElevenLabs custom-LLM relay to Gemma 4 — +1,770 | +1,770 |
| 16:29 | `f1e63c0` | SQLite event log with group commit + `/api/stats` — +527 | +527 |
| 16:33 | `8966eed` | Gemma 4 scenario generator with schema validation — +1,058 | +1,058 |
| 16:41 | `0bd9f11` | io_uring HTTP server (`SERVER=uring`) — +768 | +768 |
| 16:51 | `2b5d47f` | io_uring 42 ms stall fix — +22 | +22 |
| 16:57 | `6c9bd02` | Frontend import (Lovable subtree) + call wiring | 0 (generated) |
| 17:16 | `87146dc` | io_uring inline fast path, 1.5–2.3× Axum — +54 | +54 |
| 17:28 | `b1b1474` | Fixture corrections: 3 mismatched links, duplicate caller — +29 | +29 |
| 17:39 | `b7151e3` | Smarter debrief + Gemma round feedback after 5 cases — +428 | +428 |
| 17:50 | — | **Feature freeze** | |
| 18:10–18:40 | — | Pitch deck, spoken script, demo recording | docs only |

**≈5,970 lines** of backend, generator and fixture code were written between **15:34 and 17:39** — 2 hours 5 minutes.

## Verify it yourself

Four commands, no trust required:

```bash
# The pre-event commit: 17 files, 515 lines, zero application code
git show --stat 6492867

# Proves there is no code in it — prints nothing
git ls-tree -r --name-only 6492867 | grep -E '\.(rs|py|ts|tsx|json)$'

# First time the Rust backend existed: 22977d6, 15:34
git log --diff-filter=A --format='%h %ad %s' --date=iso -- backend/src/main.rs

# Code added per commit, in order
git log --reverse --shortstat -- backend scripts
```

Every other first appearance lands in the same window: `backend/src/db.rs` at 16:29, `scripts/generate_scenarios.py` at 16:33, `backend/src/uring.rs` at 16:41, `frontend/src/routes/index.tsx` at 16:57.

## What that means for the rules

Repo scaffolding created before the event is not built product: CI configuration, a licence, a PR template and a documentation folder don't implement anything. Everything listed above is disclosed in full so nothing has to be taken on faith — and the parts that make the trainer work (Gemma 4 scenarios, the relay, the io_uring server, the debrief logic) were all written during the event.

One thing to confirm: the hackathon's official start time. The first commit is at 11:39, roughly two hours before kickoff, and the gap above shows nothing was committed until 15:26. Add the exact start time here if the organisers publish one.
