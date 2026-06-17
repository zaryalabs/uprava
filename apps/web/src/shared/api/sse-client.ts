import { apiBase } from "./http-client";
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
  const source = new EventSource(url);
  source.addEventListener("cortex.event", (event) => {
    onEvent(JSON.parse((event as MessageEvent).data) as EventEnvelope);
  });
  source.onerror = () => onError();
  return () => source.close();
}
