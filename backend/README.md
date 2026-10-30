# backend/

Python 3.12 + FastAPI. Gemma 4 (Gemini API) and ElevenLabs are called from here. Scaffold this at kickoff (MLH: no project code before the event).

Suggested layout:

```
backend/
  pyproject.toml     # uv init; deps: fastapi, uvicorn, google-genai, elevenlabs, python-dotenv
  app/
    main.py          # FastAPI app, /health, CORS: allow_origin_regex for *.lovable.app / *.lovableproject.com + CORS_ORIGINS
    ai.py            # Gemma 4 calls
    voice.py         # ElevenLabs calls
```

Run (once scaffolded):

```bash
cd backend && uv sync && uv run uvicorn app.main:app --reload --port 8000
# expose it at https://hacktober-api.lpuehringer.dev (named tunnel, see docs/ARCHITECTURE.md):
cloudflared tunnel run --url http://localhost:8000 hacktober
```
