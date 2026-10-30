#!/usr/bin/env python3
"""Ask Gemma 4 for training scenarios and write them to backend/fixtures/scenarios.json.

One-off generator for #6. Standard library only. Reads MODEL_BASE_URL, GEMINI_API_KEY and
GEMMA_MODEL from the environment or the repo's .env (see .env.example).

Hand-written fixtures (ids not starting with "gen_") are kept; earlier "gen_" scenarios are replaced.

    python3 scripts/generate_scenarios.py              # all slots (~20 scenarios)
    python3 scripts/generate_scenarios.py --limit 1    # refusal check: one scam email, printed, nothing written
    python3 scripts/generate_scenarios.py --dry-run    # generate + validate, don't write
"""

import argparse
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "backend" / "fixtures" / "scenarios.json"

COMPANY = """\
Fictional company: Donauwerk Logistik GmbH, a logistics company in Linz, Austria, ~400 staff.
Real domain: donauwerk.example. Internal services: login.donauwerk.example (Microsoft 365 sign-in),
intranet.donauwerk.example (intranet), invoices.donauwerk.example (invoice portal), it.donauwerk.example (IT tickets).
Phone numbers: +43 732 000 xxxx (main switchboard +43 732 000 1000). Internal extensions are 4 digits.
People:
- Katharina Weber, CEO (katharina.weber@donauwerk.example)
- Thomas Huber, CFO (thomas.huber@donauwerk.example)
- Sabine Gruber, Accounting (sabine.gruber@donauwerk.example)
- Markus Leitner, IT Service Desk lead (it-desk@donauwerk.example, ext. 1234)
- Elena Novak, HR (elena.novak@donauwerk.example)
- Florian Bauer, Warehouse Linz-Hafen shift lead (florian.bauer@donauwerk.example)
- Julia Steiner, Facilities (julia.steiner@donauwerk.example)
External partners: Linz AG (electricity, linzag.example), Spedition Moser (carrier, spedition-moser.example),
Raiffeisen Bank (raiffeisen.example), Office supplies Kaiser (kaiser-buero.example).
Normal processes:
- Payments over 1,000 EUR need four-eyes approval in the invoice portal. Bank detail changes are confirmed by calling
  the supplier back on the number already on file, never the number in the request.
- IT never asks for passwords or MFA codes, by phone or email. Password resets go through a ticket at it.donauwerk.example.
- HR sends payslips and policy updates via the intranet, not as attachments that need macros.
- Anything confidential is shared via the intranet, not by private email or phone."""

SYSTEM = """\
You write scenarios for a security-awareness TRAINING SIMULATOR. Employees of a fictional company practise spotting
phishing emails and social-engineering phone calls in a safe sandbox. Everything is fictional: only the reserved
.example top-level domain is used, no real people, no real brands' real domains. Realistic scams are needed so staff
learn to recognise them; every scam is labelled with its red flags and the evidence, which is shown to the trainee
in a debrief. Legitimate scenarios are equally important: they must look normal and follow the company's processes.

""" + COMPANY + """

Output ONLY one JSON object, no markdown fences, no commentary. Schema (camelCase keys exactly as shown):
{
  "channel": "email" | "call",
  "isScam": boolean,
  "redFlags": [ { "type": RED_FLAG, "evidence": "exact quote or concrete fact from the scenario" } ],   // [] if legit
  "safeSignals": [ "why this is legitimate, concrete" ],                                               // [] if scam
  "correctAction": "report" if scam else "comply",
  "email": {                       // only if channel == "email", otherwise omit
    "fromName": string, "fromAddress": string, "subject": string,
    "bodyMarkdown": string,        // reference links inline as [text](link:l1); short, 60-150 words, signed
    "links": [ { "id": "l1", "shownHref": string, "realHref": string,
                 "landing": { "template": "m365_login" | "intranet_page", "brand": string, "trap": boolean } } ],
    "attachments": [ { "id": "a1", "filename": string, "kind": "pdf" | "docx_macro",
                       "preview": { "title": string, "text": string }, "trap": boolean } ]
  },
  "call": {                        // only if channel == "call", otherwise omit
    "callerName": string, "callerId": string, "goal": string,
    "target": short snake_case id of what the caller wants (e.g. "mfa_code", "password", "bank_change", "none"),
    "openingLine": string,        // first thing the caller says, 1-2 sentences
    "personaPrompt": string       // instructions for the voice AI playing the caller; see rules
  }
}
RED_FLAG is one of: urgency, authority, credential_request, payment_change, sender_mismatch, link_mismatch,
risky_attachment, secrecy, process_bypass.

Rules:
- Every domain and email address uses the .example TLD. Scam look-alikes are e.g. donauwerk-support.example, not real TLDs.
- trap is true only for scam links/attachments (credential page, macro document). Legit scenarios have no traps.
- A docx_macro attachment's preview asks to "Enable content". pdf previews show plausible document text.
- Scams have 2-5 redFlags; if a link's shown and real domains differ, include link_mismatch; each evidence quotes the email/opening line or names the concrete mismatch.
- Legit scenarios have 2-3 safeSignals and may be mildly unusual, but follow the processes above.
- personaPrompt (calls): second person ("You are ..."), describes who the caller pretends to be, what they want,
  how they react to pushback, and ends with: "Keep every reply to 1-2 short sentences. Never break character."
  For scams also: "Never admit it is a scam." For legit calls: the caller accepts callbacks and tickets happily.
- English only, including the openingLine. Today is 30 October 2026: use 2026 dates and years.
- Fields the trainee sees (names, addresses, subject, body, filenames, previews, callerName, openingLine) must never
  hint that it is a scam: no "(fake)", "(impersonator)", "phishing" and so on."""

# (channel, isScam, theme). Varied so the pool doesn't repeat itself.
SLOTS = [
    ("email", True, "Fake Microsoft 365 password-expiry notice with a look-alike login link (m365_login trap)"),
    ("email", True, "CEO fraud: 'Katharina Weber' from a private-looking address asks for an urgent, secret transfer"),
    ("email", True, "Supplier Spedition Moser 'changed bank details', invoice PDF attached, asks to update before payment run"),
    ("email", True, "HR 'salary adjustment' document that needs macros enabled (docx_macro trap)"),
    ("email", True, "Fake intranet notice: new parking/holiday policy, link to look-alike intranet login (intranet_page trap)"),
    ("email", True, "Shared document notification from a look-alike domain asking to sign in to view a 'Q4 bonus list'"),
    ("email", False, "Accounting asks for second approval of a normal invoice in the invoice portal (pdf attached)"),
    ("email", False, "IT announces planned maintenance window this weekend, link to the real intranet page"),
    ("email", False, "HR reminds about the annual training, link to the intranet, no attachment"),
    ("email", False, "Facilities shares the updated fire-drill plan as a PDF"),
    ("email", False, "Warehouse shift lead asks to swap a shift next week, plain email, no links"),
    ("email", False, "CFO asks to prepare the monthly cost report by Friday, link to intranet folder"),
    ("call", True, "Fake IT support says the account is hacked and asks for the MFA code"),
    ("call", True, "Caller posing as Raiffeisen bank fraud team asks to confirm a transfer by reading out a TAN"),
    ("call", True, "Caller claiming to be the CEO's assistant needs gift cards bought urgently for a client, keep it secret"),
    ("call", True, "Fake supplier Spedition Moser calls to 'confirm' new bank details before the payment run"),
    ("call", False, "IT Service Desk calls back about a ticket the user opened, asks them to restart their laptop, no secrets"),
    ("call", False, "Spedition Moser dispatcher calls to reschedule a pickup slot at the Linz-Hafen warehouse"),
    ("call", False, "HR calls to remind about a missing signature on the training form, suggests doing it on the intranet"),
    ("call", False, "Facilities calls to arrange access for the window cleaners next Tuesday"),
]

RED_FLAGS = {"urgency", "authority", "credential_request", "payment_change", "sender_mismatch",
             "link_mismatch", "risky_attachment", "secrecy", "process_bypass"}
TEMPLATES = {"m365_login", "intranet_page"}
KINDS = {"pdf", "docx_macro"}
GIVEAWAY_RE = re.compile(r"scam|phish|impersonat|\bfake\b|fraudster|attacker|\b20(1\d|2[0-5])\b|\bHallo\b|\bIch\b", re.I)
DOMAIN_RE = re.compile(r"(?:https?://|@)([A-Za-z0-9.-]+\.[A-Za-z]{2,})")


def hostname(url):
    return urlparse(url if "//" in url else "https://" + url).hostname


def load_env():
    env = ROOT / ".env"
    if env.exists():
        for line in env.read_text().splitlines():
            if "=" in line and not line.lstrip().startswith("#"):
                k, v = line.split("=", 1)
                os.environ.setdefault(k.strip(), v.strip())


def chat(messages):
    base = os.environ.get("MODEL_BASE_URL", "https://generativelanguage.googleapis.com/v1beta/openai").rstrip("/")
    body = {"model": os.environ.get("GEMMA_MODEL", "gemma-4-26b-a4b-it"), "messages": messages, "temperature": 0.9}
    req = urllib.request.Request(
        base + "/chat/completions",
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + os.environ.get("GEMINI_API_KEY", "")},
    )
    for attempt in range(3):
        try:
            with urllib.request.urlopen(req, timeout=180) as r:
                return json.load(r)["choices"][0]["message"]["content"] or ""
        except urllib.error.HTTPError as e:
            msg = e.read().decode(errors="replace")[:500]
            if e.code in (429, 500, 502, 503) and attempt < 2:
                time.sleep(5 * (attempt + 1))
                continue
            sys.exit(f"HTTP {e.code} from model: {msg}")
        except (urllib.error.URLError, TimeoutError) as e:
            if attempt == 2:
                sys.exit(f"model unreachable: {e}")
            time.sleep(5)


def extract_json(text):
    text = re.sub(r"<thought>.*?</thought>", "", text, flags=re.S)
    start, end = text.find("{"), text.rfind("}")
    if start < 0 or end < start:
        raise ValueError("no JSON object in the reply")
    return json.loads(text[start:end + 1])


def strings(v):
    if isinstance(v, str):
        yield v
    elif isinstance(v, dict):
        for x in v.values():
            yield from strings(x)
    elif isinstance(v, list):
        for x in v:
            yield from strings(x)


def validate(s, channel, scam):
    """Mirror backend/src/scenario.rs plus the #6 content rules. Returns a list of problems."""
    errs = []

    def need(obj, key, typ, where):
        if not isinstance(obj.get(key), typ):
            errs.append(f"{where}.{key} missing or not {typ.__name__}")
            return False
        return True

    if s.get("channel") != channel:
        errs.append(f"channel must be {channel!r}")
    if s.get("isScam") is not scam:
        errs.append(f"isScam must be {str(scam).lower()}")
    if s.get("correctAction") != ("report" if scam else "comply"):
        errs.append("correctAction must be " + ("report" if scam else "comply"))
    if need(s, "redFlags", list, "scenario"):
        for f in s["redFlags"]:
            if not isinstance(f, dict) or f.get("type") not in RED_FLAGS or not isinstance(f.get("evidence"), str):
                errs.append(f"bad redFlag {f!r}")
        if scam and not 2 <= len(s["redFlags"]) <= 5:
            errs.append("scams need 2-5 redFlags")
        if not scam and s["redFlags"]:
            errs.append("legit scenarios have redFlags: []")
    if not isinstance(s.get("safeSignals", []), list) or not all(isinstance(x, str) for x in s.get("safeSignals", [])):
        errs.append("safeSignals must be a list of strings")
    elif not scam and not s.get("safeSignals"):
        errs.append("legit scenarios need safeSignals")

    other = "call" if channel == "email" else "email"
    if s.get(other) is not None:
        errs.append(f"{other} must be omitted for a {channel}")
    if not isinstance(s.get(channel), dict):
        return errs + [f"{channel} object missing"]
    c = s[channel]

    if channel == "email":
        for k in ("fromName", "fromAddress", "subject", "bodyMarkdown"):
            need(c, k, str, "email")
        link_ids = set()
        c["links"] = c.get("links") or []
        c["attachments"] = c.get("attachments") or []
        for l in c["links"]:
            for k in ("id", "shownHref", "realHref"):
                need(l, k, str, "link")
            link_ids.add(l.get("id"))
            land = l.get("landing")
            if not isinstance(land, dict) or land.get("template") not in TEMPLATES \
                    or not isinstance(land.get("brand"), str) or not isinstance(land.get("trap"), bool):
                errs.append(f"link {l.get('id')}: landing needs template in {sorted(TEMPLATES)}, brand, trap")
            elif land["trap"] and not scam:
                errs.append("legit scenarios have no trap links")
            shown, real = (hostname(l.get(k) or "") for k in ("shownHref", "realHref"))
            if shown != real:
                if not scam:
                    errs.append(f"link {l.get('id')}: legit links go where they say ({shown} vs {real})")
                elif "link_mismatch" not in {f.get("type") for f in s.get("redFlags") or [] if isinstance(f, dict)}:
                    errs.append(f"link {l.get('id')} shows {shown} but goes to {real}: add a link_mismatch redFlag")
        for a in c["attachments"]:
            for k in ("id", "filename"):
                need(a, k, str, "attachment")
            if a.get("kind") not in KINDS:
                errs.append(f"attachment kind must be one of {sorted(KINDS)}")
            p = a.get("preview")
            if not isinstance(p, dict) or not isinstance(p.get("title"), str) or not isinstance(p.get("text"), str):
                errs.append("attachment.preview needs title and text")
            if not isinstance(a.get("trap"), bool):
                errs.append("attachment.trap must be a boolean")
            elif a["trap"] and not scam:
                errs.append("legit scenarios have no trap attachments")
        refs = set(re.findall(r"\(link:([^)]+)\)", c.get("bodyMarkdown") or ""))
        for ref in refs - link_ids:
            errs.append(f"body references link:{ref} but no such link")
        for lid in link_ids - refs:
            errs.append(f"link {lid} must appear in bodyMarkdown as [text](link:{lid})")
    else:
        for k in ("callerName", "callerId", "goal", "target", "openingLine", "personaPrompt"):
            need(c, k, str, "call")
        c["voiceId"] = os.environ.get("ELEVENLABS_VOICE_ID") or "TODO"

    public = {k: v for k, v in c.items() if k not in ("goal", "target", "personaPrompt")}
    for text in strings(public):
        if GIVEAWAY_RE.search(text):
            errs.append(f"visible text gives the answer away or is outdated/non-English: {text[:80]!r}")
    for text in strings(s):
        for host in DOMAIN_RE.findall(text):
            if not host.lower().rstrip(".").endswith(".example"):
                errs.append(f"non-.example domain {host!r}")
    return errs


def generate(slot_no, channel, scam, theme):
    kind = ("SCAM" if scam else "LEGITIMATE") + " " + channel
    messages = [
        {"role": "system", "content": SYSTEM},
        {"role": "user", "content": f"Write one {kind} scenario. Theme: {theme}"},
    ]
    for attempt in range(4):
        reply = chat(messages)
        try:
            s = extract_json(reply)
            errs = validate(s, channel, scam) if isinstance(s, dict) else ["reply is not a JSON object"]
        except ValueError as e:
            errs = [f"invalid JSON: {e}"]
        except (AttributeError, TypeError) as e:
            errs = [f"wrong structure: {e}"]
        if not errs:
            s["id"] = f"gen_{channel}_{slot_no:02d}"
            # Key order like the hand-written fixtures.
            order = ["id", "channel", "isScam", "redFlags", "safeSignals", "correctAction", "email", "call"]
            return {k: s[k] for k in order if k in s}
        print(f"  slot {slot_no} attempt {attempt + 1}: {'; '.join(errs)[:300]}", file=sys.stderr)
        messages += [
            {"role": "assistant", "content": reply},
            {"role": "user", "content": "Fix these problems and output the full corrected JSON object only:\n- "
                                        + "\n- ".join(errs)},
        ]
    return None


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--limit", type=int, help="only the first N slots; with N=1 prints the result and writes nothing")
    ap.add_argument("--dry-run", action="store_true", help="validate but don't write the fixtures file")
    args = ap.parse_args()
    load_env()
    if not os.environ.get("GEMINI_API_KEY") and "generativelanguage" in os.environ.get("MODEL_BASE_URL", "generativelanguage"):
        sys.exit("GEMINI_API_KEY is not set (put it in .env)")

    slots = SLOTS[: args.limit] if args.limit else SLOTS
    out, failed = [], []
    for i, (channel, scam, theme) in enumerate(slots, 1):
        print(f"[{i}/{len(slots)}] {'scam' if scam else 'legit'} {channel}: {theme}", file=sys.stderr)
        s = generate(i, channel, scam, theme)
        (out.append(s) if s else failed.append(i))

    if args.limit == 1:
        print(json.dumps(out, indent=2, ensure_ascii=False))
        return
    print(f"{len(out)} valid, {len(failed)} failed {failed or ''}", file=sys.stderr)
    if args.dry_run or not out:
        return
    kept = [s for s in json.loads(FIXTURES.read_text()) if not s["id"].startswith("gen_")]
    FIXTURES.write_text(json.dumps(kept + out, indent=2, ensure_ascii=False) + "\n")
    print(f"wrote {len(kept)} hand-written + {len(out)} generated scenarios to {FIXTURES.relative_to(ROOT)}",
          file=sys.stderr)


if __name__ == "__main__":
    main()
