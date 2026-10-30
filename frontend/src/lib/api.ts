import { API_URL } from "@/config";

export type Channel = "email" | "call";
export type Landing = { template: "m365_login" | "intranet_page" | "invoice_portal" | "parcel_tracking"; brand: string };
export type EmailLink = { id: string; shownHref: string; realHref: string; landing: Landing };
export type Attachment = {
  id: string;
  filename: string;
  kind: "pdf" | "docx_macro" | "double_extension" | "html_login" | "zip_password";
  preview: { title: string; text: string };
};
export type Scenario =
  | { id: string; channel: "email"; email: { fromName: string; fromAddress: string; subject: string; bodyMarkdown: string; links: EmailLink[]; attachments: Attachment[] } }
  | { id: string; channel: "call"; call: { callerName: string; callerId: string; openingLine: string; voiceId: string } };
export type Flag = { type: string; evidence: string };
export type Decision = {
  correct: boolean; isScam: boolean; correctAction: string;
  redFlags: Flag[]; caught: Flag[]; missed: Flag[]; safeSignals: string[]; explanation: string;
};
export type EventType = "open" | "hover_link" | "click_link" | "open_attachment" | "submit_credentials" | "enable_content";
export type Action = "comply" | "report" | "fell_for_it";

async function req<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(API_URL + path, { ...init, headers: { "Content-Type": "application/json", ...init?.headers } });
  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new Error(body?.error ?? `Request failed (${res.status})`);
  }
  return res.status === 204 ? (undefined as T) : res.json();
}

export const api = {
  startRound: () => req<{ roundId: string; scenarios: { id: string; channel: Channel }[] }>("/api/round", { method: "POST" }),
  scenario: (id: string) => req<Scenario>(`/api/scenario/${id}`),
  event: (id: string, type: EventType, detail: Record<string, string> = {}) =>
    req<void>(`/api/scenario/${id}/event`, { method: "POST", body: JSON.stringify({ type, detail }) }).catch(() => {}),
  decide: (id: string, action: Action) =>
    req<Decision>(`/api/scenario/${id}/decision`, { method: "POST", body: JSON.stringify({ action }) }),
};
