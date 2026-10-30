import { useEffect, useRef, useState } from "react";
import { ConversationProvider, useConversation } from "@elevenlabs/react";
import { Button } from "@/components/ui/button";
import { CallTranscript } from "@/components/call-transcript";
import type { Action, Scenario } from "@/lib/api";
import type { Turn } from "@/lib/call.functions";

type Props = { s: Extract<Scenario, { channel: "call" }>; decide: (action: Action) => void; locked: boolean };
type Phase = "ringing" | "connecting" | "listening" | "caller" | "ended" | "error";

// This SDK version requires a provider. Scope it to the scenario so pending
// connections and their microphone are also closed when the scenario unmounts.
export function CallView(props: Props) {
  return <ConversationProvider><AgentCall {...props} /></ConversationProvider>;
}

function microphoneError(error: unknown) {
  if (error instanceof DOMException) {
    if (error.name === "NotAllowedError" || error.name === "SecurityError") return "Microphone access was denied. Allow microphone access in your browser, then try again.";
    if (error.name === "NotFoundError") return "No microphone was found. Connect a microphone, then try again.";
    if (error.name === "NotReadableError") return "Your microphone is unavailable or in use by another app. Check it, then try again.";
  }
  return error instanceof Error ? error.message : "Could not connect the call. Please try again.";
}

function AgentCall({ s, decide, locked }: Props) {
  const [phase, setPhase] = useState<Phase>("ringing");
  const [turns, setTurns] = useState<Turn[]>([]);
  const [partialTranscript, setPartialTranscript] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [voiceActive, setVoiceActive] = useState(false);
  const alive = useRef(true);
  const active = useRef(false);
  const starting = useRef(false);
  const attempt = useRef(0);
  const failed = useRef(false);
  const permissionStream = useRef<MediaStream | null>(null);
  const events = useRef(new Map<string, number>());

  const releaseMic = () => {
    permissionStream.current?.getTracks().forEach((track) => track.stop());
    permissionStream.current = null;
  };

  const conversation = useConversation({
    onConnect: () => {
      if (!alive.current || !active.current) return;
      starting.current = false;
      setPhase("listening");
    },
    onModeChange: ({ mode }) => {
      if (alive.current && active.current && !failed.current) setPhase(mode === "speaking" ? "caller" : "listening");
    },
    onVadScore: ({ vadScore }) => {
      if (alive.current && active.current) setVoiceActive(vadScore > 0.5);
    },
    onMessage: ({ message, role, event_id }) => {
      if (!alive.current || !active.current || !message.trim()) return;
      if (role === "user") setPartialTranscript("");
      const key = `${role}:${event_id}`;
      const existing = events.current.get(key);
      setTurns((current) => {
        const turn: Turn = { role: role === "user" ? "employee" : "caller", text: message };
        if (existing !== undefined) return current.map((t, i) => i === existing ? turn : t);
        return [...current, turn];
      });
      if (existing === undefined) events.current.set(key, events.current.size);
    },
    onIncomingEvent: (event: unknown) => {
      if (!alive.current || !active.current || !event || typeof event !== "object" || !("type" in event) || event.type !== "tentative_user_transcript") return;
      if (!("tentative_user_transcription_event" in event)) return;
      const payload = event.tentative_user_transcription_event;
      if (payload && typeof payload === "object" && "user_transcript" in payload && typeof payload.user_transcript === "string") setPartialTranscript(payload.user_transcript);
    },
    onAgentResponseCorrection: ({ event_id, corrected_agent_response }) => {
      if (!alive.current || !active.current) return;
      const index = events.current.get(`agent:${event_id}`);
      if (index !== undefined) setTurns((current) => current.map((t, i) => i === index ? { ...t, text: corrected_agent_response } : t));
    },
    onDisconnect: (details) => {
      releaseMic();
      starting.current = false;
      if (!alive.current || !active.current) return;
      active.current = false;
      setVoiceActive(false);
      setPartialTranscript("");
      if (details.reason === "error" || failed.current) {
        failed.current = true;
        if (details.reason === "error") setError(`Call connection failed: ${details.message || details.closeReason || "The connection was lost."}`);
        setPhase("error");
      } else {
        // An agent end_call is not evidence of compliance. Only buttons decide.
        setPhase("ended");
      }
    },
    onError: (message, context: unknown) => {
      if (!alive.current || !active.current) return;
      failed.current = true;
      active.current = false;
      starting.current = false;
      releaseMic();
      setVoiceActive(false);
      setPartialTranscript("");
      setError(context instanceof DOMException ? microphoneError(context) : `Call connection failed: ${message}`);
      setPhase("error");
      // Provider endSession also closes any in-flight start once it resolves.
      endRef.current();
    },
  });
  const endRef = useRef(conversation.endSession);
  useEffect(() => { endRef.current = conversation.endSession; }, [conversation.endSession]);

  const stop = () => {
    ++attempt.current;
    active.current = false;
    releaseMic();
    endRef.current();
  };

  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      ++attempt.current;
      active.current = false;
      releaseMic();
      endRef.current();
    };
  }, []);

  useEffect(() => {
    if (locked) {
      stop();
      setPhase("ended");
    }
  }, [locked]); // eslint-disable-line react-hooks/exhaustive-deps

  const answer = async () => {
    if (locked || starting.current || active.current || conversation.status === "connecting") return;
    starting.current = true;
    active.current = true;
    failed.current = false;
    const id = ++attempt.current;
    setError(null);
    setTurns([]);
    setPartialTranscript("");
    events.current.clear();
    setPhase("connecting");
    try {
      if (!navigator.mediaDevices?.getUserMedia) throw new Error("Microphone access is unavailable. Use a supported browser over a secure connection.");
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      // Permission may finish after hangup, a locked decision, or unmount.
      if (!alive.current || id !== attempt.current || !active.current) {
        stream.getTracks().forEach((track) => track.stop());
        return;
      }
      permissionStream.current = stream;
      releaseMic(); // The agent SDK owns its own recording stream.
      const voiceId = s.call.voiceId?.trim();
      conversation.startSession({
        agentId: "agent_1101m66q5pz2e2zs40g5sn1ycx7t",
        connectionType: "websocket",
        dynamicVariables: { scenario_id: s.id, opening_line: s.call.openingLine },
        ...(/^[a-zA-Z0-9]{20}$/.test(voiceId ?? "") ? { overrides: { tts: { voiceId } } } : {}),
      });
      // startSession returns void in SDK 1.20.2; callbacks report completion.
    } catch (err) {
      if (!alive.current || id !== attempt.current || !active.current) return;
      active.current = false;
      failed.current = true;
      releaseMic();
      setError(microphoneError(err));
      setPhase("error");
      endRef.current();
    } finally {
      if (id !== attempt.current || !active.current) starting.current = false;
    }
  };

  const hangUp = () => {
    stop();
    setVoiceActive(false);
    setPartialTranscript("");
    setPhase("ended");
  };
  const status: Record<Phase, string> = {
    ringing: "Incoming call", connecting: "Connecting…", caller: "● Caller speaking",
    listening: "● Recording — just talk", ended: "Call ended", error: "Call disconnected",
  };
  const isActive = phase === "connecting" || phase === "caller" || phase === "listening";

  return (
    <div className="rounded-md border border-border bg-card">
      <div className="flex flex-col items-center p-8">
        <p className="stamp text-muted-foreground">{s.call.callerId}</p>
        <p className="mt-2 font-display text-4xl font-bold">{s.call.callerName}</p>
        <div className="mt-6 flex h-14 items-center gap-1">
          {Array.from({ length: 20 }).map((_, k) => (
            <span key={k} className={`w-1.5 rounded-full ${phase === "listening" ? "bg-safe" : "bg-accent"} ${phase === "caller" || (phase === "listening" && voiceActive) ? "animate-wave" : ""}`}
              style={{ height: `${18 + ((k * 37) % 38)}px`, animationDelay: `${k * 0.05}s`, opacity: isActive ? 1 : 0.3 }} />
          ))}
        </div>
        {phase === "listening" && (
          <div className="relative mt-5 flex h-16 w-16 items-center justify-center" aria-label="Recording your voice">
            <span className={`absolute inset-0 rounded-full bg-safe/30 ${voiceActive ? "animate-ping" : "animate-pulse"}`} />
            <span className="absolute inset-2 rounded-full bg-safe/40" />
            <span className="relative flex h-10 w-10 items-center justify-center rounded-full bg-safe text-safe-foreground">
              <svg viewBox="0 0 24 24" className="h-5 w-5" fill="currentColor"><path d="M12 14a3 3 0 0 0 3-3V5a3 3 0 0 0-6 0v6a3 3 0 0 0 3 3Zm5-3a5 5 0 0 1-10 0H5a7 7 0 0 0 6 6.92V21h2v-3.08A7 7 0 0 0 19 11h-2Z" /></svg>
            </span>
          </div>
        )}
        <p role="status" className={`stamp mt-4 ${phase === "listening" ? "text-safe" : "text-accent"} ${phase === "connecting" ? "animate-pulse" : ""}`}>{status[phase]}</p>
        <div className="mt-5 flex gap-2">
          {(phase === "ringing" || phase === "error") && <Button onClick={() => void answer()} disabled={locked || conversation.status === "connecting"} className="stamp h-auto rounded bg-safe px-6 py-3 text-safe-foreground">{phase === "error" ? "Try again" : "Answer call"}</Button>}
          {isActive && <Button onClick={hangUp} variant="destructive" className="stamp h-auto rounded px-4 py-2">Hang up</Button>}
        </div>
        {phase === "ringing" && <p className="mt-3 text-xs text-muted-foreground">Microphone access is needed to speak with the caller.</p>}
        {error && <p role="alert" className="mt-3 font-mono text-xs text-destructive">! {error}</p>}
      </div>
      <CallTranscript turns={turns} partialTranscript={partialTranscript} />
      {phase === "ended" && (
        <div className="border-t border-border p-5">
          <p className="stamp mb-3 text-muted-foreground">What happened on that call?</p>
          <div className="flex gap-3">
            <Button disabled={locked} onClick={() => decide("report")} variant="destructive" className="stamp h-auto flex-1 whitespace-normal rounded px-4 py-3">Report it to IT</Button>
            <Button disabled={locked} onClick={() => decide("comply")} variant="outline" className="stamp h-auto flex-1 whitespace-normal rounded border-foreground px-4 py-3">I gave them what they asked</Button>
          </div>
        </div>
      )}
    </div>
  );
}