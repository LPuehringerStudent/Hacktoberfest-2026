import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HookOptions } from "@elevenlabs/react";
import { CallView } from "./call-view";
import type { Scenario } from "@/lib/api";

const sdk = vi.hoisted(() => ({ callbacks: {} as HookOptions, start: vi.fn(), end: vi.fn() }));
vi.mock("@elevenlabs/react", () => ({
  ConversationProvider: ({ children }: { children: React.ReactNode }) => children,
  useConversation: (callbacks: HookOptions) => {
    sdk.callbacks = callbacks;
    return { startSession: sdk.start, endSession: sdk.end, status: "disconnected" };
  },
}));

const s: Extract<Scenario, { channel: "call" }> = {
  id: "gen_call_17", channel: "call", call: {
    callerName: "Markus", callerId: "+43 123", openingLine: "Calling about your ticket.", voiceId: "EXAVITQu4vr4xnSDxMaL",
  },
};
const stopTrack = vi.fn();
const mic = vi.fn();
beforeEach(() => {
  vi.clearAllMocks();
  mic.mockResolvedValue({ getTracks: () => [{ stop: stopTrack }] });
  Object.defineProperty(navigator, "mediaDevices", { configurable: true, value: { getUserMedia: mic } });
});
afterEach(cleanup);

async function start() {
  fireEvent.click(screen.getByRole("button", { name: "Answer call" }));
  await waitFor(() => expect(sdk.start).toHaveBeenCalledTimes(1));
  act(() => sdk.callbacks.onConnect?.({ conversationId: "conversation" }));
}

describe("Tripwire conversational agent", () => {
  it("forwards the approved agent, scenario variables and stock voice in normal WebSocket voice mode", async () => {
    render(<CallView s={s} decide={vi.fn()} locked={false} />);
    await start();
    expect(sdk.start).toHaveBeenCalledWith({
      agentId: "agent_1101m66q5pz2e2zs40g5sn1ycx7t", connectionType: "websocket",
      dynamicVariables: { scenario_id: "gen_call_17", opening_line: "Calling about your ticket." },
      overrides: { tts: { voiceId: "EXAVITQu4vr4xnSDxMaL" } },
    });
    expect(stopTrack).toHaveBeenCalledTimes(1);
  });

  for (const voiceId of ["", "TODO"]) {
    it(`omits the ${JSON.stringify(voiceId)} voice so the agent uses Eric`, async () => {
      render(<CallView s={{ ...s, call: { ...s.call, voiceId } }} decide={vi.fn()} locked={false} />);
      await start();
      expect(sdk.start.mock.calls[0]?.[0]).not.toHaveProperty("overrides");
    });
  }

  for (const reason of ["user", "agent"] as const) {
    it(`${reason} disconnect never records a decision without an explicit choice`, async () => {
      const decide = vi.fn();
      render(<CallView s={s} decide={decide} locked={false} />);
      await start();
      act(() => sdk.callbacks.onDisconnect?.({ reason }));
      expect(decide).not.toHaveBeenCalled();
      fireEvent.click(screen.getByRole("button", { name: "Report it to IT" }));
      expect(decide).toHaveBeenCalledExactlyOnceWith("report");
    });
  }

  it("hangup closes the session without choosing compliance", async () => {
    const decide = vi.fn();
    render(<CallView s={s} decide={decide} locked={false} />);
    await start();
    fireEvent.click(screen.getByRole("button", { name: "Hang up" }));
    act(() => sdk.callbacks.onDisconnect?.({ reason: "user" }));
    expect(sdk.end).toHaveBeenCalled();
    expect(decide).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "I gave them what they asked" }));
    expect(decide).toHaveBeenCalledExactlyOnceWith("comply");
  });

  it("connection errors offer manual retry and never submit decisions, even after teardown disconnect", async () => {
    const decide = vi.fn();
    render(<CallView s={s} decide={decide} locked={false} />);
    await start();
    act(() => {
      sdk.callbacks.onError?.("Relay unavailable");
      sdk.callbacks.onDisconnect?.({ reason: "user" });
    });
    expect(decide).not.toHaveBeenCalled();
    expect(sdk.start).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    await waitFor(() => expect(sdk.start).toHaveBeenCalledTimes(2));
  });

  it("abnormal transport disconnect allows retry without a decision", async () => {
    const decide = vi.fn();
    render(<CallView s={s} decide={decide} locked={false} />);
    await start();
    act(() => sdk.callbacks.onDisconnect?.({ reason: "error", message: "Network lost", context: { type: "close" } }));
    expect(decide).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    await waitFor(() => expect(sdk.start).toHaveBeenCalledTimes(2));
  });

  it("denied microphone permission does not start a session or decide", async () => {
    mic.mockRejectedValue(new DOMException("Denied", "NotAllowedError"));
    const decide = vi.fn();
    render(<CallView s={s} decide={decide} locked={false} />);
    fireEvent.click(screen.getByRole("button", { name: "Answer call" }));
    await screen.findByRole("button", { name: "Try again" });
    expect(sdk.start).not.toHaveBeenCalled();
    expect(decide).not.toHaveBeenCalled();
  });

  it("guards concurrent starts before permission and before onConnect", async () => {
    let resolve: ((stream: unknown) => void) | undefined;
    mic.mockImplementation(() => new Promise((r) => { resolve = r; }));
    render(<CallView s={s} decide={vi.fn()} locked={false} />);
    const button = screen.getByRole("button", { name: "Answer call" });
    act(() => { button.click(); button.click(); });
    expect(mic).toHaveBeenCalledTimes(1);
    await act(async () => resolve?.({ getTracks: () => [{ stop: stopTrack }] }));
    expect(sdk.start).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("button", { name: "Answer call" })).toBeNull();
  });

  for (const cancellation of ["hangup", "unmount", "locked"] as const) {
    it(`cancels permission-pending starts on ${cancellation} and stops the late microphone`, async () => {
      let resolve: ((stream: unknown) => void) | undefined;
      mic.mockImplementation(() => new Promise((r) => { resolve = r; }));
      const decide = vi.fn();
      const view = render(<CallView s={s} decide={decide} locked={false} />);
      fireEvent.click(screen.getByRole("button", { name: "Answer call" }));
      if (cancellation === "unmount") view.unmount();
      else if (cancellation === "locked") view.rerender(<CallView s={s} decide={decide} locked />);
      else fireEvent.click(screen.getByRole("button", { name: "Hang up" }));
      await act(async () => resolve?.({ getTracks: () => [{ stop: stopTrack }] }));
      expect(stopTrack).toHaveBeenCalledTimes(1);
      expect(sdk.start).not.toHaveBeenCalled();
      expect(sdk.end).toHaveBeenCalled();
      expect(decide).not.toHaveBeenCalled();
    });
  }

  it("ends an SDK-pending start on unmount and ignores late callbacks", async () => {
    const decide = vi.fn();
    const { unmount } = render(<CallView s={s} decide={decide} locked={false} />);
    fireEvent.click(screen.getByRole("button", { name: "Answer call" }));
    await waitFor(() => expect(sdk.start).toHaveBeenCalledTimes(1));
    unmount();
    act(() => { sdk.callbacks.onConnect?.({ conversationId: "late" }); sdk.callbacks.onDisconnect?.({ reason: "agent" }); });
    expect(sdk.end).toHaveBeenCalled();
    expect(decide).not.toHaveBeenCalled();
  });

  it("a locked decision closes an active session and prevents restart", async () => {
    const decide = vi.fn();
    const view = render(<CallView s={s} decide={decide} locked={false} />);
    await start();
    view.rerender(<CallView s={s} decide={decide} locked />);
    expect(sdk.end).toHaveBeenCalled();
    act(() => sdk.callbacks.onDisconnect?.({ reason: "user" }));
    expect(decide).not.toHaveBeenCalled();
    expect(sdk.start).toHaveBeenCalledTimes(1);
  });

  it("SDK microphone startup failure allows retry without a decision", async () => {
    const decide = vi.fn();
    render(<CallView s={s} decide={decide} locked={false} />);
    fireEvent.click(screen.getByRole("button", { name: "Answer call" }));
    await waitFor(() => expect(sdk.start).toHaveBeenCalledTimes(1));
    act(() => sdk.callbacks.onError?.("Permission denied", new DOMException("Denied", "NotAllowedError")));
    expect(decide).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    await waitFor(() => expect(sdk.start).toHaveBeenCalledTimes(2));
  });
});