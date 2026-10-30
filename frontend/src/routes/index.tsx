import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState, type ReactNode } from "react";
import { api, type Action, type Attachment, type Decision, type EmailLink, type Scenario } from "@/lib/api";
import { CallView } from "@/components/call-view";

export const Route = createFileRoute("/")({
  head: () => ({
    meta: [
      { title: "Tripwire — Scam Awareness Training for Donauwerk" },
      { name: "description", content: "Spot phishing emails and survive simulated scam phone calls, coached by Gemma." },
      { property: "og:title", content: "Tripwire — Scam Awareness Training" },
      { property: "og:description", content: "Spot phishing emails and survive simulated scam phone calls, coached by Gemma." },
      { property: "og:type", content: "website" },
      { name: "twitter:card", content: "summary_large_image" },
    ],
  }),
  component: Index,
});

const FLAG_LABEL: Record<string, string> = {
  urgency: "Urgency", authority: "Authority pressure", credential_request: "Credential request",
  payment_change: "Payment change", sender_mismatch: "Sender mismatch", link_mismatch: "Link mismatch",
  risky_attachment: "Risky attachment", secrecy: "Secrecy", process_bypass: "Process bypass",
};

function Index() {
  const [ids, setIds] = useState<string[] | null>(null);
  const [step, setStep] = useState(0);
  const [score, setScore] = useState({ right: 0, total: 0 });
  const [error, setError] = useState<string | null>(null);

  const start = () => {
    setError(null);
    api.startRound().then((r) => { setIds(r.scenarios.map((s) => s.id)); setStep(0); setScore({ right: 0, total: 0 }); })
      .catch((e: Error) => setError(e.message));
  };

  const done = ids && step >= ids.length;

  return (
    <div className="min-h-screen">
      <header className="border-b border-border">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
          <div className="flex items-baseline gap-3">
            <span className="font-display text-2xl font-bold tracking-tight">Tripwire<span className="text-accent">_</span></span>
            <span className="stamp text-muted-foreground">Donauwerk Logistik · Security Desk</span>
          </div>
          <div className="stamp">
            {ids && !done && <span className="mr-6 text-muted-foreground">Scenario {step + 1}/{ids.length}</span>}
            Score <span className="text-accent">{score.right}/{score.total}</span>
          </div>
        </div>
      </header>

      <main className="mx-auto max-w-6xl px-6 py-12">
        {!ids || done ? (
          <section>
            <p className="stamp text-accent">// training_file_07 · social_engineering</p>
            <h1 className="mt-3 max-w-4xl font-display text-6xl font-bold leading-[0.95] tracking-tight md:text-8xl">
              Trust nothing that <em className="font-bold text-accent">hurries</em> you.
            </h1>
            <p className="mt-6 max-w-xl text-lg text-muted-foreground">
              {done
                ? `Round complete. You got ${score.right} of ${score.total} right.`
                : "A round of real-looking emails and phone calls. Inspect links, open attachments, then decide: do it, or report it."}
            </p>
            <button onClick={start} className="stamp mt-8 rounded bg-primary px-6 py-4 text-primary-foreground hover:opacity-90">
              {done ? "Play another round →" : "Start round →"}
            </button>
            {error && <p className="mt-4 font-mono text-sm text-destructive">! {error}</p>}
          </section>
        ) : (
          <ScenarioView
            key={ids[step]}
            id={ids[step]!}
            onResult={(ok) => setScore((s) => ({ right: s.right + (ok ? 1 : 0), total: s.total + 1 }))}
            onNext={() => setStep((s) => s + 1)}
            last={step === ids.length - 1}
          />
        )}
      </main>

      <footer className="border-t border-border py-6 text-center stamp text-muted-foreground">
        Coaching by Gemma · Voices by ElevenLabs · Fictional company for training only
      </footer>
    </div>
  );
}

function ScenarioView({ id, onResult, onNext, last }: { id: string; onResult: (ok: boolean) => void; onNext: () => void; last: boolean }) {
  const [s, setS] = useState<Scenario | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [decision, setDecision] = useState<Decision | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api.scenario(id).then((x) => { setS(x); api.event(id, "open"); }).catch((e: Error) => setError(e.message));
  }, [id]);

  const decide = (a: Action) => {
    if (decision || busy) return;
    setBusy(true);
    api.decide(id, a).then((d) => { setDecision(d); onResult(d.correct); })
      .catch((e: Error) => setError(e.message)).finally(() => setBusy(false));
  };

  if (error) return <p className="font-mono text-destructive">! {error}</p>;
  if (!s) return <p className="stamp animate-pulse text-muted-foreground">Loading scenario…</p>;

  return (
    <div className="grid gap-6 lg:grid-cols-[1.4fr_1fr]">
      {s.channel === "email" ? <EmailView s={s} decide={decide} locked={!!decision} /> : <CallView s={s} decide={decide} locked={!!decision} />}
      <aside className="rounded-md border border-border bg-card p-6">
        <p className="stamp text-muted-foreground">Gemma coach</p>
        {!decision ? (
          <p className="mt-4 font-display text-2xl font-medium leading-snug">
            {s.channel === "email" ? "Check the sender, hover the links, look at attachments. Then decide." : "Listen for pressure. Would the real IT team ask this?"}
          </p>
        ) : (
          <DecisionPanel d={decision} onNext={onNext} last={last} />
        )}
      </aside>
    </div>
  );
}

function DecisionPanel({ d, onNext, last }: { d: Decision; onNext: () => void; last: boolean }) {
  const caught = new Set(d.caught.map((f) => f.evidence));
  return (
    <>
      <p className={`mt-4 font-display text-3xl font-bold ${d.correct ? "text-safe" : "text-destructive"}`}>{d.correct ? "Good call." : "Not quite."}</p>
      <p className="mt-2 text-sm text-muted-foreground">{d.explanation}</p>
      {d.redFlags.length > 0 && (
        <ul className="mt-5 space-y-3">
          {d.redFlags.map((f) => (
            <li key={f.evidence} className="text-sm">
              <span className={`stamp ${caught.has(f.evidence) ? "text-safe" : "text-accent"}`}>
                {caught.has(f.evidence) ? "✓ " : "▸ "}{FLAG_LABEL[f.type] ?? f.type}
              </span>
              <p className="mt-1 text-muted-foreground">{f.evidence}</p>
            </li>
          ))}
        </ul>
      )}
      {d.safeSignals.length > 0 && (
        <ul className="mt-5 space-y-2">
          {d.safeSignals.map((f) => <li key={f} className="text-sm"><span className="stamp text-safe">✓ safe </span>{f}</li>)}
        </ul>
      )}
      <button onClick={onNext} className="stamp mt-6 rounded bg-primary px-4 py-3 text-primary-foreground">{last ? "See results →" : "Next scenario →"}</button>
    </>
  );
}

function ActionBar({ decide, locked, comply }: { decide: (a: Action) => void; locked: boolean; comply: string }) {
  return (
    <div className="flex gap-3 border-t border-border p-5">
      <button disabled={locked} onClick={() => decide("report")} className="stamp flex-1 rounded bg-destructive px-4 py-3 text-destructive-foreground hover:opacity-90 disabled:opacity-40">Report</button>
      <button disabled={locked} onClick={() => decide("comply")} className="stamp flex-1 rounded border border-foreground px-4 py-3 hover:bg-secondary disabled:opacity-40">{comply}</button>
    </div>
  );
}

function renderBody(md: string, links: EmailLink[], onHover: (l: EmailLink | null) => void, onClick: (l: EmailLink) => void) {
  const out: ReactNode[] = [];
  const re = /\[([^\]]+)\]\(link:([^)]+)\)|\*\*([^*]+)\*\*/g;
  let last = 0, m: RegExpExecArray | null, k = 0;
  while ((m = re.exec(md))) {
    out.push(md.slice(last, m.index));
    if (m[3]) out.push(<b key={k++}>{m[3]}</b>);
    else {
      const l = links.find((x) => x.id === m![2]);
      out.push(l ? (
        <a key={k++} href={l.shownHref} title={l.shownHref} onClick={(e) => { e.preventDefault(); onClick(l); }}
          onMouseEnter={() => onHover(l)} onMouseLeave={() => onHover(null)} className="text-accent underline underline-offset-4">{m[1]}</a>
      ) : m[1]);
    }
    last = re.lastIndex;
  }
  out.push(md.slice(last));
  return out;
}

function EmailView({ s, decide, locked }: { s: Extract<Scenario, { channel: "email" }>; decide: (a: Action) => void; locked: boolean }) {
  const e = s.email;
  const [hover, setHover] = useState<EmailLink | null>(null);
  const [landing, setLanding] = useState<EmailLink | null>(null);
  const [att, setAtt] = useState<Attachment | null>(null);

  const onHover = (l: EmailLink | null) => { setHover(l); if (l) api.event(s.id, "hover_link", { linkId: l.id }); };
  const onClick = (l: EmailLink) => { setLanding(l); api.event(s.id, "click_link", { linkId: l.id }); };
  const openAtt = (a: Attachment) => { setAtt(a); api.event(s.id, "open_attachment", { attachmentId: a.id }); };

  return (
    <article className="relative rounded-md border border-border bg-card shadow-[0_0_60px_-20px_var(--color-accent)]">
      <div className="space-y-1 border-b border-border p-5 font-mono text-sm">
        <div><span className="text-muted-foreground">From: </span>{e.fromName} &lt;{e.fromAddress}&gt;</div>
        <div><span className="text-muted-foreground">Subject: </span><b>{e.subject}</b></div>
      </div>
      <div className="whitespace-pre-wrap p-5 text-[15px] leading-relaxed">{renderBody(e.bodyMarkdown, e.links, onHover, onClick)}</div>
      {e.attachments.length > 0 && (
        <div className="flex flex-wrap gap-2 px-5 pb-5">
          {e.attachments.map((a) => (
            <button key={a.id} onClick={() => openAtt(a)} className="rounded border border-border bg-secondary px-3 py-2 font-mono text-xs hover:border-accent">📎 {a.filename}</button>
          ))}
        </div>
      )}
      <div className="border-t border-border bg-background px-5 py-1.5 font-mono text-xs text-muted-foreground">{hover ? hover.realHref : "\u00a0"}</div>
      <ActionBar decide={decide} locked={locked} comply="Do it" />

      {landing && <Modal onClose={() => setLanding(null)} title={landing.realHref}><LandingPage l={landing} onSubmit={() => { api.event(s.id, "submit_credentials", { linkId: landing.id }); setLanding(null); decide("fell_for_it"); }} /></Modal>}
      {att && (
        <Modal onClose={() => setAtt(null)} title={att.filename}>
          <p className="font-display text-xl font-bold">{att.preview.title}</p>
          <p className="mt-3 text-sm text-muted-foreground">{att.preview.text}</p>
          {(att.kind === "docx_macro" || att.kind === "html_login" || att.kind === "double_extension") && (
            <button onClick={() => { api.event(s.id, "enable_content", { attachmentId: att.id }); setAtt(null); decide("fell_for_it"); }}
              className="stamp mt-5 rounded bg-safe px-4 py-2 text-safe-foreground">Enable content</button>
          )}
        </Modal>
      )}
    </article>
  );
}

function LandingPage({ l, onSubmit }: { l: EmailLink; onSubmit: () => void }) {
  const login = l.landing.template === "m365_login" || l.landing.template === "intranet_page";
  return (
    <div>
      <p className="font-display text-2xl font-bold">{l.landing.brand}</p>
      {login ? (
        <form onSubmit={(e) => { e.preventDefault(); onSubmit(); }} className="mt-4 space-y-3">
          <input placeholder="you@donauwerk.example" className="w-full rounded border border-input bg-background px-3 py-2 text-sm" />
          <input type="password" placeholder="Password" className="w-full rounded border border-input bg-background px-3 py-2 text-sm" />
          <button className="stamp w-full rounded bg-primary px-4 py-3 text-primary-foreground">Sign in</button>
        </form>
      ) : (
        <p className="mt-3 text-sm text-muted-foreground">
          {l.landing.template === "invoice_portal" ? "Invoice details loaded. Close this window and decide in the email." : "Tracking details loaded. Close this window and decide in the email."}
        </p>
      )}
    </div>
  );
}

function Modal({ title, children, onClose }: { title: string; children: ReactNode; onClose: () => void }) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 p-6 backdrop-blur-sm" onClick={onClose}>
      <div className="w-full max-w-md rounded-md border border-border bg-card" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between border-b border-border px-4 py-2 font-mono text-xs text-muted-foreground">
          <span className="truncate">{title}</span>
          <button onClick={onClose} className="ml-4 hover:text-foreground">✕</button>
        </div>
        <div className="p-6">{children}</div>
      </div>
    </div>
  );
}

