import { apiBase } from "./config";
import { logClientEvent } from "../logging/client-logger";
import type { EventEnvelope } from "../protocol/types";

export function openSessionStream(
  sessionThreadId: string,
  afterSeq: number,
  onEvent: (event: EventEnvelope) => void,
  onError: () => void,
) {
  const url = `${apiBase}/sessions/${encodeURIComponent(
    sessionThreadId,
  )}/stream?after_seq=${afterSeq}`;
  const source = new EventSource(url, { withCredentials: true });
  source.addEventListener("uprava.event", (event) => {
    try {
      onEvent(JSON.parse((event as MessageEvent).data) as EventEnvelope);
    } catch (error) {
      logClientEvent(
        "error",
        "web.sse",
        "session stream payload parse failed",
        {
          session_thread_id: sessionThreadId,
          after_seq: afterSeq,
          error: error instanceof Error ? error.message : String(error),
        },
      );
      onError();
    }
  });
  source.addEventListener("uprava.reload", (event) => {
    logClientEvent("warn", "web.sse", "session stream requested reload", {
      session_thread_id: sessionThreadId,
      after_seq: afterSeq,
      reason: (event as MessageEvent).data,
    });
    onError();
  });
  source.onerror = () => {
    logClientEvent("warn", "web.sse", "session stream error", {
      session_thread_id: sessionThreadId,
      after_seq: afterSeq,
    });
    onError();
  };
  return () => source.close();
}
