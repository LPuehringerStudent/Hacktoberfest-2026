# Two-minute demo runbook

Working handoff for [#20](https://github.com/LPuehringerStudent/Hacktoberfest-2026/issues/20), not evidence that a video has been recorded or a submission made. Use alongside [SUBMISSION.md](SUBMISSION.md) and the team's README work.

## Before recording

- Open the published app linked from the README; verify that its build includes the working call screen. Development fallback: [Lovable preview](https://id-preview--27704a8a-5cbe-462a-b04c-5e8a42825448.lovable.app). Do not present a preview as the production deployment.
- Allow microphone access, use headphones, and make a short local recording that captures **both microphone and application audio**. Play it back before recording the demo.
- Close notifications, terminals containing environment variables, dashboard secrets and unrelated tabs. Use fictional training data only; never speak a real password or verification code.
- Rehearse an email, a live call and their debriefs. Rounds are random: do not promise a particular scenario appears first. Have the required screens ready, or clearly label transitions between rounds in the recording.
- Check the ElevenLabs quota before rehearsing: the agent was configured with a 20-call daily limit, two concurrent calls and a 180-second session maximum. Do not raise those limits without the account owner's approval.
- Have the public repository and a working backup video ready. No backup video exists merely because this checklist is present.

## Recording script (target 1:50, maximum 2:00)

| Time | Show | Say / do |
|---|---|---|
| 0:00–0:10 | App inbox | “Hi, this is our demo for the MLH x YAIL Hacktoberfest HackDay. We built a safe phishing simulator for spotting suspicious emails and phone calls.” |
| 0:10–0:35 | One email, link hover, then decision and debrief | Point out one concrete clue, hover the link to expose its real destination, and choose Report or Do it. Show the returned evidence rather than simply asserting the answer. |
| 0:35–1:15 | Incoming call and live conversation | Let the opening finish. Ask a short question relevant to the caller, then pause for the spoken reply. Explain: “ElevenLabs handles speech; our Rust relay gives self-hosted Gemma 4 the scenario's private caller persona.” |
| 1:15–1:35 | Hang up, explicit decision, call debrief | Explain your decision and show the evidence. Agent hangup alone is not proof of compliance or failure. If a completed round score is available, briefly show it; do not fabricate a five-scenario result. |
| 1:35–1:50 | App and repository / architecture | “Gemma also generated our validated, labelled scenario fixtures. We learned to separate speech turn detection from model response latency: changing the turn detector fixed calls that recorded speech but never replied.” |

Leave ten seconds for variable response latency. If the call stalls, stop the recording and investigate; do not describe a prerecorded or text-only response as a live microphone test. Avoid attempting to demonstrate automatic `end_call` on cue: it was verified separately, but immediate goal-triggered termination is not guaranteed in every scenario.

## Known-working voice setup

Handoff from [#8](https://github.com/LPuehringerStudent/Hacktoberfest-2026/issues/8#issuecomment-6527748742):

- Agent: `agent_1101m66q5pz2e2zs40g5sn1ycx7t`.
- Runtime-tested custom LLM **base URL**: `https://hacktober-api.lpuehringer.dev/v1`; backend chat endpoint: `/v1/chat/completions`. Do not append the endpoint twice.
- Dynamic variables: `scenario_id` from the scenario ID and `opening_line` from `scenario.call.openingLine`.
- First message protected with `disable_first_message_interruptions: true`; turn detector `turn_v2`; ASR `scribe_realtime`; closed-model backup disabled.
- Stock voices: Eric (default), Sarah and George. Missing / placeholder fixture voice IDs fall back to Eric in the call UI.
- Real-microphone browser success was confirmed by Mohamad after a reproducible streamed-PCM speech test produced a live transcript and spoken Gemma reply. `end_call` was verified in a separate text-mode integration test.
- Stop recording mutes transmitted microphone audio; Hang up ends the session and releases the microphone. Neither operation should silently submit a training decision.

If the opening plays but speech gets no response, check the live turn detector setting before rewriting the frontend or replacing Gemma. If an error occurs, retry rather than recording a compliance decision. Credentials remain in workspace secrets / server environment variables, never in demo materials.

## Remaining submission handoff

The following are **pending checks**, not completed or independently verified eligibility claims:

- [ ] Coordinate the final product name, team names, screenshots, learning notes and AI-tool disclosures with the existing README author; avoid competing README rewrites.
- [ ] Verify the published app and backend, and reconcile the frontend import / sync state with the repository maintainer.
- [ ] Record the flow above with audible application speech, then upload the approved recording and add its URL to the README.
- [ ] Check the current event rules, category requirements and eligibility with the organizer. In particular, resolve the ElevenLabs registration cutoff noted in [CHALLENGES.md](CHALLENGES.md), and confirm whether the Gemma category accepts the deployed self-hosted path rather than assuming the older Gemini-API wording is satisfied.
- [ ] Verify public repository access and licenses, including the frontend's source location. Do not claim a separate public Lovable repository exists without its URL.
- [ ] Have the authorized teammate complete the event's submission flow for each eligible category. Record the confirmation links / receipt privately with the team; do not mark submitted based only on an open form.
- [ ] Close #20 only after the README, recording and actual submissions are complete.

No publishing, category entry, registration, quota increase or submission is authorized by this document. Ask the human before performing those actions.
