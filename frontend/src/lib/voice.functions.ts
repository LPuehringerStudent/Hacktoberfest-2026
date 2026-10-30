import { createServerFn } from "@tanstack/react-start";

const FALLBACK_VOICE = "CwhRBWXzGAHq8TQ4Fs17"; // Roger

export const speak = createServerFn({ method: "POST" })
  .inputValidator((d: { text: string; voiceId?: string }) => {
    if (typeof d?.text !== "string" || !d.text.trim() || d.text.length > 1000) throw new Error("Invalid text");
    return d;
  })
  .handler(async ({ data }) => {
    const key = process.env['ELEVENLABS_API_KEY'];
    if (!key) return { error: "Voice is not configured." as const };
    const voice = data.voiceId && /^[A-Za-z0-9]{10,40}$/.test(data.voiceId) ? data.voiceId : FALLBACK_VOICE;
    const res = await fetch(`https://api.elevenlabs.io/v1/text-to-speech/${voice}?output_format=mp3_44100_128`, {
      method: "POST",
      headers: { "xi-api-key": key, "Content-Type": "application/json" },
      body: JSON.stringify({ text: data.text, model_id: "eleven_turbo_v2_5", voice_settings: { stability: 0.4, similarity_boost: 0.75 } }),
    });
    if (!res.ok) {
      console.error("ElevenLabs TTS failed", res.status, await res.text());
      return { error: `Voice service error (${res.status})` };
    }
    const bytes = new Uint8Array(await res.arrayBuffer());
    let bin = "";
    for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
    return { audio: btoa(bin) };
  });
