import type { QueryClient } from "@tanstack/react-query";

import { queryKeys } from "../../shared/api/query-keys";
import type { EventEnvelope, SessionDetail } from "../../shared/protocol/types";
import { applySessionEvent } from "./apply-session-event";

export type SessionStreamCacheResult =
  | { kind: "applied" }
  | {
      kind: "reloaded";
      reason: "missing-cache" | "sequence-gap";
      expectedSeq?: number;
      receivedSeq?: number;
    };

export async function applySessionStreamEventToCache(
  queryClient: QueryClient,
  sessionThreadId: string,
  event: EventEnvelope,
): Promise<SessionStreamCacheResult> {
  let result: SessionStreamCacheResult = {
    kind: "reloaded",
    reason: "missing-cache",
  };

  queryClient.setQueryData<SessionDetail>(
    queryKeys.session(sessionThreadId),
    (current) => {
      if (!current) return current;
      const projection = applySessionEvent(current, event);
      if (projection.kind === "gap") {
        result = {
          kind: "reloaded",
          reason: "sequence-gap",
          expectedSeq: projection.expectedSeq,
          receivedSeq: projection.receivedSeq,
        };
        return current;
      }
      result = { kind: "applied" };
      return projection.detail;
    },
  );

  if (result.kind === "reloaded") {
    await invalidateSessionSnapshots(queryClient, sessionThreadId, true);
    return result;
  }

  await invalidateSessionSnapshots(queryClient, sessionThreadId, false);
  return result;
}

export async function invalidateSessionSnapshots(
  queryClient: QueryClient,
  sessionThreadId: string,
  includeSession: boolean,
) {
  await Promise.all([
    includeSession
      ? queryClient.invalidateQueries({
          queryKey: queryKeys.session(sessionThreadId),
        })
      : Promise.resolve(),
    queryClient.invalidateQueries({
      queryKey: queryKeys.agentProjection(sessionThreadId),
    }),
    queryClient.invalidateQueries({
      queryKey: queryKeys.artifactTree(sessionThreadId),
    }),
    queryClient.invalidateQueries({ queryKey: queryKeys.inventory }),
  ]);
}
