import { afterEach, describe, expect, it, vi } from "vitest";

import { openSessionStream } from "./sse-client";

describe("openSessionStream", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("routes malformed protocol payloads to the reload/error path", () => {
    const eventSource = new MockEventSource();
    class EventSourceStub {
      constructor() {
        return eventSource;
      }
    }
    vi.stubGlobal("EventSource", EventSourceStub);
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(null, { status: 204 })),
    );
    const onEvent = vi.fn();
    const onError = vi.fn();

    const close = openSessionStream("session-1", 0, onEvent, onError);

    eventSource.emit("uprava.event", {
      event_id: "event-1",
      command_id: null,
      actor_ref: {},
      scope_ref: {},
      node_id: null,
      runtime_session_id: null,
      session_thread_id: "session-1",
      turn_id: null,
      seq: "not-a-number",
      kind: "runtime.ready",
      happened_at: "2026-07-10T00:00:00Z",
      source_refs: [],
      evidence_refs: [],
      cause_refs: [],
      result_refs: [],
      payload: {},
    });

    expect(onEvent).not.toHaveBeenCalled();
    expect(onError).toHaveBeenCalledTimes(1);

    close();
    expect(eventSource.closed).toBe(true);
  });
});

class MockEventSource {
  readonly listeners = new Map<string, (event: MessageEvent) => void>();
  closed = false;
  onerror: (() => void) | null = null;

  addEventListener(type: string, listener: (event: MessageEvent) => void) {
    this.listeners.set(type, listener);
  }

  emit(type: string, payload: unknown) {
    this.listeners.get(type)?.({
      data: JSON.stringify(payload),
    } as MessageEvent);
  }

  close() {
    this.closed = true;
  }
}
