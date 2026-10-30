import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { CallTranscript } from "./call-transcript";
import type { Turn } from "@/lib/call.functions";

afterEach(cleanup);

describe("Call transcript follows speech", () => {
  for (const role of ["caller", "employee"] as const) {
    it(`scrolls to the bottom when the ${role} adds a turn`, () => {
      const turns: Turn[] = [{ role: "caller", text: "Hello" }];
      const { rerender } = render(<CallTranscript turns={turns} partialTranscript="" />);
      const transcript = screen.getByRole("log");
      Object.defineProperty(transcript, "scrollHeight", { configurable: true, value: 960 });
      transcript.scrollTop = 0;
      rerender(<CallTranscript turns={[...turns, { role, text: "A new turn" }]} partialTranscript="" />);
      expect(transcript.scrollTop).toBe(960);
    });
  }

  it("follows employee speech before it is committed", () => {
    const turns: Turn[] = [{ role: "caller", text: "Hello" }];
    const { rerender } = render(<CallTranscript turns={turns} partialTranscript="" />);
    const transcript = screen.getByRole("log");
    Object.defineProperty(transcript, "scrollHeight", { configurable: true, value: 1200 });
    transcript.scrollTop = 0;
    rerender(<CallTranscript turns={turns} partialTranscript="I will verify" />);
    expect(transcript.scrollTop).toBe(1200);
  });
});