# Tripwire frontend (Lovable mirror)

This folder mirrors the Tripwire frontend developed in Lovable. Make UI changes in Lovable, then update this mirror through a pull request. This upload does not configure automatic future sync.

## Development

```sh
bun install --frozen-lockfile
bun run dev
bun run test
bun run build
```

The application uses TanStack Start, React, TypeScript and Tailwind CSS. Scenario/event/decision requests use the team Rust backend. Live calls use the public ElevenLabs conversational agent, which routes to the Rust relay and self-hosted Gemma. No keys are required in the browser. Legacy server voice helpers remain in source but are not used by the call screen.

Published app: https://hactoberfest.lovable.app (may lag this source; this upload does not publish).

## Validation and disclosure

AI-assisted frontend implementation and export. Latest verification: 20 tests passed, typecheck passed and Lovable automatic build passed. A browser with simulated microphone received real agent opening text/audio; manual hangup submitted no decision. Physical microphone roundtrip and voice-mode automatic end_call remain unverified.

Secrets, repository metadata, dependencies and generated build/cache files are excluded.
