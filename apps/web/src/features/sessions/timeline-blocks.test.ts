import { describe, expect, it } from "vitest";

import { buildSessionTimelineBlocks, blockFromEvent } from "./timeline-blocks";
import type { EventEnvelope, SessionDetail } from "../../shared/protocol/types";
import {
  getTimelineBlockRenderer,
  registeredTimelineBlockTypes,
} from "../../workbench/blocks/TimelineBlockRenderer";

describe("session timeline blocks", () => {
  it("maps messages and approval events to typed ui blocks", () => {
    const blocks = buildSessionTimelineBlocks(detailWithApproval());

    expect(blocks.map((item) => item.block.type)).toEqual([
      "core.assistant-message",
      "core.approval-request",
    ]);
    expect(blocks[0].block.source_refs[0]).toMatchObject({
      kind: "event",
      event_id: "event-message",
    });
    expect(blocks[1].approvalId).toBe("approval-1");
    expect(blocks[1].block.primary_ref).toEqual({
      kind: "approval",
      approval_id: "approval-1",
    });
    expect(blocks[1].block.actions).toEqual(["approval.resolve"]);
  });

  it("renders newer dialogue blocks after older ones", () => {
    const detail = detailWithApproval();
    const userMessage = {
      message_id: "message-user",
      session_thread_id: "session-1",
      turn_id: "turn-1",
      role: "user" as const,
      content: "Run checks",
      created_at: "2026-06-17T00:00:00Z",
      completed_at: "2026-06-17T00:00:00Z",
      source_event_id: null,
    };
    const assistantMessage = {
      ...detail.messages[0],
      message_id: "message-assistant",
      turn_id: "turn-1",
      created_at: "2026-06-17T00:00:02Z",
      completed_at: "2026-06-17T00:00:02Z",
    };

    const blocks = buildSessionTimelineBlocks({
      ...detail,
      messages: [assistantMessage, userMessage],
      events: [
        {
          ...detail.events[0],
          turn_id: "turn-1",
          seq: 3,
          happened_at: "2026-06-17T00:00:02Z",
        },
        {
          ...detail.events[1],
          turn_id: "turn-1",
          seq: 2,
          happened_at: "2026-06-17T00:00:01Z",
        },
      ],
    });

    expect(blocks.map((item) => item.block.block_id)).toEqual([
      "message:message-user",
      "event:event-approval.requested",
      "message:message-assistant",
    ]);
  });

  it("maps known event families to registered renderer types", () => {
    const event = eventWithPayload("resource.snapshot.updated", {
      git_branch: "main",
    });

    const item = blockFromEvent(event);

    expect(item.block.type).toBe("core.resource-snapshot");
    expect(getTimelineBlockRenderer(item.block.type)?.allowedSurfaces).toEqual([
      "session.timeline",
    ]);
  });

  it("groups provider activity events into one turn block before the assistant message", () => {
    const providerMessage = {
      ...eventWithPayload("provider.message.completed", {
        content: "Done",
      }),
      event_id: "event-assistant",
      turn_id: "turn-1",
      seq: 5,
      happened_at: "2026-06-17T00:00:04Z",
    };
    const detail = {
      ...detailWithApproval(),
      messages: [
        {
          message_id: "message-user",
          session_thread_id: "session-1",
          turn_id: "turn-1",
          role: "user" as const,
          content: "Run checks",
          created_at: "2026-06-17T00:00:00Z",
          completed_at: "2026-06-17T00:00:00Z",
          source_event_id: null,
        },
        {
          message_id: "message-assistant",
          session_thread_id: "session-1",
          turn_id: "turn-1",
          role: "assistant" as const,
          content: "Done",
          created_at: "2026-06-17T00:00:04Z",
          completed_at: "2026-06-17T00:00:04Z",
          source_event_id: "event-assistant",
        },
      ],
      events: [
        {
          ...eventWithPayload("turn.started", {}),
          event_id: "event-turn-started",
          turn_id: "turn-1",
          seq: 2,
          happened_at: "2026-06-17T00:00:01Z",
        },
        {
          ...eventWithPayload("provider.activity", {
            provider_event_type: "item.completed",
            provider_item_type: "command_execution",
            status: "completed",
            summary: "make c",
            raw_event: {
              type: "item.completed",
              unknown_future_field: true,
            },
          }),
          event_id: "event-activity-1",
          turn_id: "turn-1",
          seq: 3,
          happened_at: "2026-06-17T00:00:02Z",
        },
        {
          ...eventWithPayload("provider.activity", {
            provider_event_type: "stderr",
            status: "warning",
            summary: "warning",
          }),
          event_id: "event-activity-2",
          turn_id: "turn-1",
          seq: 4,
          happened_at: "2026-06-17T00:00:03Z",
        },
        providerMessage,
        {
          ...eventWithPayload("turn.completed", {}),
          event_id: "event-turn-completed",
          turn_id: "turn-1",
          seq: 6,
          happened_at: "2026-06-17T00:00:05Z",
        },
      ],
    };

    const blocks = buildSessionTimelineBlocks(detail);

    expect(blocks.map((item) => item.block.block_id)).toEqual([
      "message:message-user",
      "turn-activity:turn-1",
      "message:message-assistant",
    ]);
    expect(blocks[1].block.type).toBe("core.turn-activity");
    expect(blocks[1].block.data).toMatchObject({
      turnId: "turn-1",
      eventCount: 2,
      commandCount: 1,
      warningErrorCount: 1,
      completed: true,
    });
  });

  it("keeps the v01 renderer registry explicit", () => {
    expect(registeredTimelineBlockTypes()).toContain("core.turn-activity");
    expect(registeredTimelineBlockTypes()).toContain("core.approval-request");
    expect(registeredTimelineBlockTypes()).toContain("core.unknown");
  });
});

function detailWithApproval(): SessionDetail {
  const messageEvent = eventWithPayload("provider.message.completed", {
    content: "Done",
  });
  const approvalEvent = eventWithPayload("approval.requested", {
    approval_id: "approval-1",
    prompt: "Allow command?",
  });

  return {
    session: {
      session_thread_id: "session-1",
      project_placement_id: "placement-1",
      runtime_session_id: "runtime-1",
      title: "Session",
      state: "active",
      runtime: {
        runtime_session_id: "runtime-1",
        provider: "fake",
        state: "blocked",
        resume_supported: true,
        degraded_reason: null,
        last_runtime_step_at: null,
      },
      message_count: 1,
      updated_at: "2026-06-17T00:00:00Z",
    },
    placement: {
      project_placement_id: "placement-1",
      project_id: null,
      node_id: "node-1",
      display_name: "cortex",
      workspace_path: "/workspace",
      state: "validated",
      resource_badges: [],
      last_validated_at: null,
    },
    messages: [
      {
        message_id: "message-1",
        session_thread_id: "session-1",
        turn_id: null,
        role: "assistant",
        content: "Done",
        created_at: "2026-06-17T00:00:00Z",
        completed_at: "2026-06-17T00:00:01Z",
        source_event_id: messageEvent.event_id,
      },
    ],
    events: [messageEvent, approvalEvent],
  };
}

function eventWithPayload(kind: string, payload: unknown): EventEnvelope {
  return {
    event_id:
      kind === "provider.message.completed" ? "event-message" : `event-${kind}`,
    command_id: null,
    actor_ref: { kind: "system" },
    scope_ref: { kind: "runtime", runtime_session_id: "runtime-1" },
    node_id: "node-1",
    runtime_session_id: "runtime-1",
    session_thread_id: "session-1",
    turn_id: null,
    seq: kind === "provider.message.completed" ? 1 : 2,
    kind,
    happened_at: "2026-06-17T00:00:00Z",
    source_refs: [],
    evidence_refs: [],
    cause_refs: [],
    result_refs: [],
    payload,
  };
}
