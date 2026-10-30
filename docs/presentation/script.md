# Tripwire — 2-minute pitch script

Everything you say, in order, on the slide it belongs to. Read it once out loud with a stopwatch before you go on.

**Slides:** 1 title · 2 video · 3 challenges · 4 under the hood · 5 close · 6 questions

**Length:** ~171 spoken words ≈ 73 s, plus the video's 46.6 s ≈ **2:00**. If the timer goes red, drop the io_uring paragraph on slide 4 — never the 62%.

---

## 1 of 6 · 0:00 — Hook

*Look at the judges, not the screen.*

> 62% of breaches involve a human being — not a bug in the code.
> Slideshows don't fix that. So we built a gym for the reflex to stop
> and check: Tripwire.

---

## 2 of 6 · 0:12 — Video

*Press play. Say nothing while it runs.*

> **▶ VIDEO — 46 s, press play ◀**

---

## 3 of 6 · 0:59 — Four challenges

*One sentence each. No elaboration.*

> Lovable built the whole frontend: an inbox anyone can use with zero
> training — hover a link and it shows where it really goes.

> Every scenario and every caller is Gemma 4 running on our own server.
> You can't send your staff's training data to someone else's cloud.

> Gemma plays the scammer too — it ends the call with a function call once
> you've given it what it wants, and JSON validation means a broken
> scenario can never ship.

> ElevenLabs makes it a real phone call. You can push back, ask for a
> callback — and it argues with you.

---

## 4 of 6 · 1:40 — The hard part

> Underneath: a hand-written io_uring server, one point five to two point
> three times faster than our Axum baseline. Same routes, one env var.

---

## 5 of 6 · 1:51 — Close

> Your firewall is fine. Your people need practice. Thank you.

---

## 6 of 6 · 1:57 — Questions?

*Stop talking. Stay on this slide. Don't say "any questions?" — let the silence ask it for you.*

---

## If a judge asks

**Where does 62% come from?**
Verizon's *2026 Data Breach Investigations Report* — 62% of breaches involve the human element. It's printed on slide 1, so they can check it.

**How is this different from KnowBe4?**
Their sims send real emails with real links to real staff. Ours is one fictional company, only reserved `.example` domains, no real site ever touched — and it's a phone call, which most trainers can't do.

**Does the AI decide whether I passed?**
No. Labels are validated against a Rust schema at generation time and stripped server-side at any depth before the payload leaves. Open devtools; the answer isn't there.

**Won't everyone just click Report?**
Half the round is legitimate. Reporting a real request costs a point — false alarms score exactly like misses.

**What was new for you?**
One concrete thing per person, not "we learned about LLMs." Shape: *"I'd never made a model hold a character under real-time streaming constraints."*

---

## One caution

Only say the "function call" sentence on slide 3 if you have **seen** the caller hang up on its own. If it's flaky, say *"and it gets you off the phone when it's done"* instead.
