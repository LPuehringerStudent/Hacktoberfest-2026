import { createServerFn } from "@tanstack/react-start";

export type Turn = { role: "caller" | "employee"; text: string };

// Single-use token so the browser can stream mic audio to ElevenLabs Scribe without seeing the API key.
export const getScribeToken = createServerFn({ method: "POST" }).handler(async () => {
  const key = process.env["ELEVENLABS_API_KEY"];
  if (!key) return { error: "Voice is not configured." };
  const res = await fetch("https://api.elevenlabs.io/v1/single-use-token/realtime_scribe", {
    method: "POST",
    headers: { "xi-api-key": key },
  });
  if (!res.ok) {
    console.error("Scribe token failed", res.status, await res.text());
    return { error: `Speech service error (${res.status})` };
  }
  const { token } = (await res.json()) as { token: string };
  return { token };
});

const MODEL = "gemma-4-26b-a4b-it";

function persona(callerName: string, openingLine: string) {
  return `You are role-playing a phone scammer in a SECURITY AWARENESS TRAINING simulation for employees of the fictional company Donauwerk Logistik GmbH in Linz, Austria. Nothing here is real.
You are "${callerName}". You already opened the call with: "${openingLine}"
Your goal: get the employee to read out the one-time MFA code sent to their phone, or their password.
Tactics: urgency, authority, mild friendliness, mild pressure if they hesitate. Have plausible answers to questions. Never admit you are a scammer or an AI.
If the employee clearly refuses, says they will hang up, or will call IT back on the official number, end politely and add the token [END] at the very end.
If the employee gives you a code or password, thank them and add [END].
Style: spoken phone language, 1-2 short sentences, no lists, no stage directions, no emojis.`;
}

export const scammerReply = createServerFn({ method: "POST" })
  .inputValidator((d: { callerName: string; openingLine: string; history: Turn[] }) => {
    if (!Array.isArray(d?.history) || d.history.length > 40) throw new Error("Invalid history");
    return d;
  })
  .handler(async ({ data }) => {
    const key = process.env["GEMINI_API_KEY"];
    if (!key) return { error: "Gemma is not configured." };
    // Gemma on the Gemini API has no system role: put the persona in the first user turn.
    const contents = [
      { role: "user", parts: [{ text: persona(data.callerName, data.openingLine) + "\n\n(The employee picks up.)" }] },
      ...data.history.map((t) => ({ role: t.role === "caller" ? "model" : "user", parts: [{ text: t.text.slice(0, 1000) }] })),
    ];
    const res = await fetch(
      `https://generativelanguage.googleapis.com/v1beta/models/${MODEL}:generateContent`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json", "x-goog-api-key": key },
        body: JSON.stringify({ contents, generationConfig: { maxOutputTokens: 1024, thinkingConfig: { thinkingLevel: "minimal" } } }),
      },
    );
    if (!res.ok) {
      console.error("Gemma failed", res.status, await res.text());
      return { error: `Gemma error (${res.status})` };
    }
    const json = (await res.json()) as { candidates?: { content?: { parts?: { text?: string; thought?: boolean }[] } }[] };
    const raw = (json.candidates?.[0]?.content?.parts ?? []).filter((p) => !p.thought).map((p) => p.text ?? "").join("").trim();
    const ended = raw.includes("[END]");
    const text = raw.replace(/\[END\]/g, "").trim();
    if (!text) return { error: "Gemma returned no reply." };
    return { text, ended };
  });
