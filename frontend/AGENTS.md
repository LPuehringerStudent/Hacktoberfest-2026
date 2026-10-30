<!-- LOVABLE:BEGIN -->
> [!IMPORTANT]
> This project is connected to [Lovable](https://lovable.dev). Avoid rewriting
> published git history — force pushing, or rebasing/amending/squashing commits
> that are already pushed — as it rewrites history on Lovable's side and the
> user will likely lose their project history.
>
> Commits you push to the connected branch sync back to Lovable and show up in
> the editor, so keep the branch in a working state.
<!-- LOVABLE:END -->

- Scenario, event and decision requests use the team Rust backend via src/config.ts and src/lib/api.ts, following docs/API.md in the Hacktoberfest-2026 repo. Live calls use the public ElevenLabs conversational agent via @elevenlabs/react: agent STT/TTS -> Rust custom-LLM relay -> self-hosted Gemma. The browser forwards scenario variables and optional stock voice only, never provider keys or relay tokens. Keep the agent custom LLM base URL https://hacktober-api.lpuehringer.dev/v1 unchanged. Legacy Lovable server voice helpers are not part of the live call path.
- Keep call transcript rendering and scroll-follow behavior in a standalone browser-safe component so speech updates can be tested without connecting voice services.

- Scope ConversationProvider to each call screen; use the installed SDK callbacks for lifecycle and provider teardown for pending starts, because startSession/endSession return void in this SDK. Only explicit employee decisions submit call outcomes.
