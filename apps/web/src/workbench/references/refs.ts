import type {
  CortexRef,
  ProjectPlacementSummary,
} from "../../shared/protocol/types";

export const INSPECT_QUERY_PARAM = "inspect";
const MAX_INSPECTOR_STACK_DEPTH = 8;

export function encodeCortexRef(ref: CortexRef) {
  return encodeURIComponent(JSON.stringify(ref));
}

export function encodeInspectorStack(refs: CortexRef[]) {
  return encodeURIComponent(JSON.stringify(refs));
}

export function decodeCortexRef(
  value: string | null | undefined,
): CortexRef | null {
  const decoded = decodeJsonValue(value);
  return isCortexRef(decoded) ? decoded : null;
}

export function decodeInspectorStack(
  value: string | null | undefined,
): CortexRef[] {
  const decoded = decodeJsonValue(value);
  if (Array.isArray(decoded)) return decoded.filter(isCortexRef);
  return isCortexRef(decoded) ? [decoded] : [];
}

export function pushInspectorRef(
  searchParams: URLSearchParams,
  ref: CortexRef,
) {
  const next = new URLSearchParams(searchParams);
  const current = decodeInspectorStack(next.get(INSPECT_QUERY_PARAM));
  const stack = sameRef(current.at(-1), ref)
    ? current
    : [...current.filter((candidate) => !sameRef(candidate, ref)), ref];
  next.set(
    INSPECT_QUERY_PARAM,
    encodeInspectorStack(stack.slice(-MAX_INSPECTOR_STACK_DEPTH)),
  );
  return next;
}

export function popInspectorRef(searchParams: URLSearchParams) {
  const next = new URLSearchParams(searchParams);
  const stack = decodeInspectorStack(next.get(INSPECT_QUERY_PARAM)).slice(
    0,
    -1,
  );
  if (stack.length === 0) {
    next.delete(INSPECT_QUERY_PARAM);
  } else {
    next.set(INSPECT_QUERY_PARAM, encodeInspectorStack(stack));
  }
  return next;
}

export function replaceInspectorStack(
  searchParams: URLSearchParams,
  refs: CortexRef[],
) {
  const next = new URLSearchParams(searchParams);
  if (refs.length === 0) {
    next.delete(INSPECT_QUERY_PARAM);
  } else {
    next.set(INSPECT_QUERY_PARAM, encodeInspectorStack(refs));
  }
  return next;
}

export function routeForRef(
  ref: CortexRef,
  options: {
    inspectorPathname?: string;
    searchParams?: URLSearchParams;
  } = {},
) {
  switch (ref.kind) {
    case "node":
      return routeFromString("/nodes", stringField(ref, "node_id"));
    case "project":
      return routeFromString("/projects", stringField(ref, "project_id"));
    case "placement":
      return routeFromString("/placements", stringField(ref, "placement_id"));
    case "workspace":
      return routeFromString("/workspaces", stringField(ref, "placement_id"));
    case "session":
      return routeFromString(
        "/sessions",
        stringField(ref, "session_thread_id"),
      );
    default:
      return routeWithInspectorRef(
        options.inspectorPathname ?? "/dashboard",
        options.searchParams ?? new URLSearchParams(),
        ref,
      );
  }
}

export function routeWithInspectorRef(
  pathname: string,
  searchParams: URLSearchParams,
  ref: CortexRef,
) {
  const next = pushInspectorRef(searchParams, ref);
  const query = next.toString();
  return query ? `${pathname}?${query}` : pathname;
}

export function projectRefForPlacement(
  placement: Pick<ProjectPlacementSummary, "project_id">,
): CortexRef | null {
  return placement.project_id
    ? { kind: "project", project_id: placement.project_id }
    : null;
}

export function workspaceRefForPlacement(
  placement: Pick<ProjectPlacementSummary, "project_placement_id">,
): CortexRef {
  return {
    kind: "workspace",
    placement_id: placement.project_placement_id,
  };
}

export function copyReferenceText(ref: CortexRef) {
  return JSON.stringify(ref, null, 2);
}

export function refTitle(ref: CortexRef) {
  return `${refKindLabel(ref)} ${refPrimaryValue(ref)}`.trim();
}

export function refKindLabel(ref: CortexRef) {
  return ref.kind.replaceAll("_", " ");
}

export function refPrimaryValue(ref: CortexRef) {
  if ("node_id" in ref && typeof ref.node_id === "string") return ref.node_id;
  if ("project_id" in ref && typeof ref.project_id === "string") {
    return ref.project_id;
  }
  if ("session_thread_id" in ref && typeof ref.session_thread_id === "string") {
    return ref.session_thread_id;
  }
  if (
    "runtime_session_id" in ref &&
    typeof ref.runtime_session_id === "string"
  ) {
    return ref.runtime_session_id;
  }
  if ("turn_id" in ref && typeof ref.turn_id === "string") return ref.turn_id;
  if ("message_id" in ref && typeof ref.message_id === "string") {
    return ref.message_id;
  }
  if ("block_id" in ref && typeof ref.block_id === "string") {
    return ref.block_id;
  }
  if ("artifact_id" in ref && typeof ref.artifact_id === "string") {
    return ref.artifact_id;
  }
  if ("event_id" in ref && typeof ref.event_id === "string") {
    return ref.event_id;
  }
  if ("command_id" in ref && typeof ref.command_id === "string") {
    return ref.command_id;
  }
  if ("approval_id" in ref && typeof ref.approval_id === "string") {
    return ref.approval_id;
  }
  if ("warning_kind" in ref && typeof ref.warning_kind === "string") {
    return ref.warning_kind;
  }
  if ("tool_call_id" in ref && typeof ref.tool_call_id === "string") {
    return ref.tool_call_id;
  }
  if ("path" in ref && typeof ref.path === "string") return ref.path;
  if ("terminal_id" in ref && typeof ref.terminal_id === "string") {
    return ref.terminal_id;
  }
  if (
    "terminal_command_id" in ref &&
    typeof ref.terminal_command_id === "string"
  ) {
    return ref.terminal_command_id;
  }
  if ("diff_id" in ref && typeof ref.diff_id === "string") return ref.diff_id;
  if ("check_run_id" in ref && typeof ref.check_run_id === "string") {
    return ref.check_run_id;
  }
  if ("edit_id" in ref && typeof ref.edit_id === "string") return ref.edit_id;
  if ("trace_event_id" in ref && typeof ref.trace_event_id === "string") {
    return ref.trace_event_id;
  }
  if ("external_id" in ref && typeof ref.external_id === "string") {
    return ref.external_id;
  }
  if ("ref_type" in ref && typeof ref.ref_type === "string") {
    return ref.ref_type;
  }
  if ("placement_id" in ref && typeof ref.placement_id === "string") {
    return ref.placement_id;
  }
  return "";
}

function routeFromString(prefix: string, value: string | null) {
  return value ? `${prefix}/${encodeURIComponent(value)}` : null;
}

function stringField(ref: CortexRef, field: string) {
  const value = (ref as Record<string, unknown>)[field];
  return typeof value === "string" && value.length > 0 ? value : null;
}

export function sameRef(left: CortexRef | null | undefined, right: CortexRef) {
  if (!left) return false;
  return copyReferenceText(left) === copyReferenceText(right);
}

function decodeJsonValue(value: string | null | undefined): unknown {
  if (!value) return null;
  const candidates = [value];
  try {
    const decoded = decodeURIComponent(value);
    if (decoded !== value) candidates.unshift(decoded);
  } catch {
    // Leave malformed URI sequences to JSON parsing below.
  }
  for (const candidate of candidates) {
    try {
      return JSON.parse(candidate) as unknown;
    } catch {
      // Try the next encoding form.
    }
  }
  return null;
}

function isCortexRef(value: unknown): value is CortexRef {
  return (
    typeof value === "object" &&
    value !== null &&
    "kind" in value &&
    typeof value.kind === "string"
  );
}
