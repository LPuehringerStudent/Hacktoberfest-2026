import { useEffect, useRef } from "react";
import type { Turn } from "@/lib/call.functions";

export function CallTranscript({ turns, partialTranscript }: { turns: Turn[]; partialTranscript: string }) {
  const transcriptRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const transcript = transcriptRef.current;
    if (transcript) transcript.scrollTop = transcript.scrollHeight;
  }, [turns, partialTranscript]);

  if (turns.length === 0) return null;

  return (
    <div ref={transcriptRef} role="log" aria-label="Call transcript" className="max-h-80 space-y-3 overflow-y-auto border-t border-border p-5">
      {turns.map((t, k) => (
        <div key={k} className={`border-l-2 pl-3 text-[15px] ${t.role === "caller" ? "border-accent" : "border-safe"}`}>
          <span className={`stamp ${t.role === "caller" ? "text-accent" : "text-safe"}`}>{t.role === "caller" ? "Caller" : "You"}</span>
          <p className="mt-1">{t.text}</p>
        </div>
      ))}
      {partialTranscript && (
        <div className="border-l-2 border-safe/50 pl-3 text-[15px] text-muted-foreground"><span className="stamp">You</span><p className="mt-1">{partialTranscript}…</p></div>
      )}
    </div>
  );
}