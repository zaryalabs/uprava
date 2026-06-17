import { describe, expect, it } from "vitest";

import type { CortexRef } from "../../shared/protocol/types";
import {
  decodeCortexRef,
  decodeInspectorStack,
  encodeCortexRef,
  INSPECT_QUERY_PARAM,
  popInspectorRef,
  pushInspectorRef,
  refTitle,
} from "./refs";

describe("reference helpers", () => {
  it("round-trips a single cortex ref", () => {
    const ref: CortexRef = { kind: "message", message_id: "message-1" };

    expect(decodeCortexRef(encodeCortexRef(ref))).toEqual(ref);
    expect(refTitle(ref)).toBe("message message-1");
  });

  it("round-trips reserved future refs through inspector URLs", () => {
    const refs: CortexRef[] = [
      { kind: "file", placement_id: "placement-1", path: "src/main.rs" },
      {
        kind: "file_range",
        placement_id: "placement-1",
        path: "src/main.rs",
        range: { start_line: 1, end_line: 3 },
      },
      {
        kind: "terminal",
        terminal_id: "terminal-1",
        placement_id: "placement-1",
      },
      {
        kind: "terminal_command",
        terminal_command_id: "terminal-command-1",
        terminal_id: "terminal-1",
      },
      {
        kind: "terminal_output_range",
        terminal_command_id: "terminal-command-1",
        range: { start_line: 5, end_line: 7 },
      },
      { kind: "diff_hunk", diff_id: "diff-1", hunk_id: "hunk-1" },
      {
        kind: "check_result",
        check_run_id: "check-1",
        failure_id: "failure-1",
      },
      {
        kind: "tool_call",
        tool_call_id: "tool-call-1",
      },
      {
        kind: "external_entity",
        integration_kind: "github",
        external_id: "pull-1",
      },
      {
        kind: "unknown",
        ref_type: "future.ref",
        locator: { id: "future-1" },
      },
    ];
    const params = refs.reduce(
      (current, ref) => pushInspectorRef(current, ref),
      new URLSearchParams(),
    );

    expect(refs.map((ref) => decodeCortexRef(encodeCortexRef(ref)))).toEqual(
      refs,
    );
    expect(decodeInspectorStack(params.get(INSPECT_QUERY_PARAM))).toEqual(
      refs.slice(-8),
    );
    expect(refTitle(refs[0])).toBe("file src/main.rs");
    expect(refTitle(refs[refs.length - 1])).toBe("unknown future.ref");
  });

  it("keeps inspector refs as a stack without duplicating the active ref", () => {
    const sessionRef: CortexRef = {
      kind: "session",
      session_thread_id: "session-1",
    };
    const eventRef: CortexRef = {
      kind: "event",
      event_id: "event-1",
      scope_ref: sessionRef,
      seq: 7,
    };
    const params = pushInspectorRef(new URLSearchParams(), sessionRef);
    const withEvent = pushInspectorRef(params, eventRef);
    const withoutDuplicate = pushInspectorRef(withEvent, eventRef);

    expect(
      decodeInspectorStack(withoutDuplicate.get(INSPECT_QUERY_PARAM)),
    ).toEqual([sessionRef, eventRef]);
    expect(
      decodeInspectorStack(
        popInspectorRef(withoutDuplicate).get(INSPECT_QUERY_PARAM),
      ),
    ).toEqual([sessionRef]);
  });
});
