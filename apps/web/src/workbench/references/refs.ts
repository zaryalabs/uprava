import type {
  UpravaRef,
  ProjectPlacementSummary,
} from "../../shared/protocol/types";

export const INSPECT_QUERY_PARAM = "inspect";
const MAX_INSPECTOR_STACK_DEPTH = 8;
const REFERENCE_KEY_VERSION = "r1";
const MAX_REFERENCE_KEY_CHARS = 4_096;
const MAX_REFERENCE_JSON_CHARS = 3_072;

export function encodeUpravaRef(ref: UpravaRef) {
  return encodeReferenceKey(ref);
}

export function encodeInspectorStack(refs: UpravaRef[]) {
  return refs
    .slice(-MAX_INSPECTOR_STACK_DEPTH)
    .map(encodeReferenceKey)
    .join("~");
}

export function decodeUpravaRef(
  value: string | null | undefined,
): UpravaRef | null {
  return decodeReferenceKey(value);
}

export function decodeInspectorStack(
  value: string | null | undefined,
): UpravaRef[] {
  if (
    !value ||
    value.length > MAX_REFERENCE_KEY_CHARS * MAX_INSPECTOR_STACK_DEPTH
  ) {
    return [];
  }
  return value
    .split("~")
    .slice(-MAX_INSPECTOR_STACK_DEPTH)
    .map(decodeReferenceKey)
    .filter((ref): ref is UpravaRef => ref !== null);
}

export function pushInspectorRef(
  searchParams: URLSearchParams,
  ref: UpravaRef,
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
  refs: UpravaRef[],
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
  ref: UpravaRef,
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
      return routeFromString("/workspaces", stringField(ref, "placement_id"));
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
  ref: UpravaRef,
) {
  const next = pushInspectorRef(searchParams, ref);
  const query = next.toString();
  return query ? `${pathname}?${query}` : pathname;
}

export function projectRefForPlacement(
  placement: Pick<ProjectPlacementSummary, "project_id">,
): UpravaRef | null {
  return placement.project_id
    ? { kind: "project", project_id: placement.project_id }
    : null;
}

export function workspaceRefForPlacement(
  placement: Pick<ProjectPlacementSummary, "project_placement_id">,
): UpravaRef {
  return {
    kind: "workspace",
    placement_id: placement.project_placement_id,
  };
}

export function copyReferenceText(ref: UpravaRef) {
  return JSON.stringify(ref, null, 2);
}

export function refTitle(ref: UpravaRef) {
  return `${refKindLabel(ref)} ${refPrimaryValue(ref)}`.trim();
}

export function refKindLabel(ref: UpravaRef) {
  return ref.kind.replaceAll("_", " ");
}

export function refPrimaryValue(ref: UpravaRef) {
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

function stringField(ref: UpravaRef, field: string) {
  const value = (ref as Record<string, unknown>)[field];
  return typeof value === "string" && value.length > 0 ? value : null;
}

export function sameRef(left: UpravaRef | null | undefined, right: UpravaRef) {
  if (!left) return false;
  return (
    JSON.stringify(canonicalize(left)) === JSON.stringify(canonicalize(right))
  );
}

function encodeReferenceKey(ref: UpravaRef) {
  if (!isUpravaRef(ref)) {
    throw new Error("Cannot encode an invalid Uprava reference");
  }
  const json = JSON.stringify(canonicalize(ref));
  if (json.length > MAX_REFERENCE_JSON_CHARS) {
    throw new Error("Uprava reference exceeds the URL size limit");
  }
  const bytes = new TextEncoder().encode(json);
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return `${REFERENCE_KEY_VERSION}.${btoa(binary)
    .replaceAll("+", "-")
    .replaceAll("/", "_")
    .replace(/=+$/u, "")}`;
}

function decodeReferenceKey(
  value: string | null | undefined,
): UpravaRef | null {
  if (!value || value.length > MAX_REFERENCE_KEY_CHARS) return null;
  const prefix = `${REFERENCE_KEY_VERSION}.`;
  if (!value.startsWith(prefix)) return null;
  const encoded = value.slice(prefix.length);
  if (!/^[A-Za-z0-9_-]+$/u.test(encoded)) return null;
  try {
    const padded = encoded.replaceAll("-", "+").replaceAll("_", "/");
    const binary = atob(padded.padEnd(Math.ceil(padded.length / 4) * 4, "="));
    const bytes = Uint8Array.from(binary, (character) =>
      character.charCodeAt(0),
    );
    const json = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
    if (json.length > MAX_REFERENCE_JSON_CHARS) return null;
    const decoded = JSON.parse(json) as unknown;
    return isUpravaRef(decoded) ? decoded : null;
  } catch {
    return null;
  }
}

function isUpravaRef(value: unknown): value is UpravaRef {
  if (!isRecord(value) || !boundedString(value.kind, 80)) return false;
  const requiredByKind: Record<string, string[]> = {
    node: ["node_id"],
    project: ["project_id"],
    placement: ["placement_id"],
    workspace: ["placement_id"],
    session: ["session_thread_id"],
    runtime: ["runtime_session_id"],
    turn: ["turn_id"],
    message: ["message_id"],
    block: ["block_id"],
    artifact: ["artifact_id"],
    event: ["event_id"],
    command: ["command_id"],
    approval: ["approval_id"],
    warning: ["warning_kind"],
    tool_call: ["tool_call_id"],
    file: ["placement_id", "path"],
    file_range: ["placement_id", "path"],
    terminal: ["terminal_id", "placement_id"],
    terminal_command: ["terminal_command_id"],
    terminal_output_range: ["terminal_command_id"],
    diff_hunk: ["diff_id", "hunk_id"],
    check_result: ["check_run_id"],
    workspace_edit: ["edit_id"],
    trace_event: ["trace_event_id"],
    external_entity: ["integration_kind", "external_id"],
    unknown: ["ref_type"],
  };
  const required = requiredByKind[value.kind];
  if (!required)
    return value.kind.startsWith("extension.") && isBoundedJson(value, 0);
  return (
    required.every((field) => boundedString(value[field], 1_024)) &&
    isBoundedJson(value, 0)
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function boundedString(value: unknown, max: number): value is string {
  return typeof value === "string" && value.length > 0 && value.length <= max;
}

function isBoundedJson(value: unknown, depth: number): boolean {
  if (depth > 6) return false;
  if (value == null || typeof value === "boolean" || typeof value === "number")
    return true;
  if (typeof value === "string") return value.length <= 2_048;
  if (Array.isArray(value))
    return (
      value.length <= 32 &&
      value.every((item) => isBoundedJson(item, depth + 1))
    );
  if (!isRecord(value) || Object.keys(value).length > 32) return false;
  return Object.entries(value).every(
    ([key, item]) => key.length <= 80 && isBoundedJson(item, depth + 1),
  );
}

function canonicalize(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonicalize);
  if (!isRecord(value)) return value;
  return Object.fromEntries(
    Object.keys(value)
      .sort()
      .map((key) => [key, canonicalize(value[key])]),
  );
}
