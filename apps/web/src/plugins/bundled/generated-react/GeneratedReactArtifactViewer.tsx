import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  type ReactNode,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import { coreApi } from "../../../shared/api/http-client";
import { queryKeys } from "../../../shared/api/query-keys";
import type {
  GeneratedUiArtifactPayload,
  GeneratedUiRuntimeDetail,
  UpravaRef,
} from "../../../shared/protocol/types";
import { generatedUiArtifactPayloadSchema } from "../../../shared/protocol/validators";
import { useOpenReference } from "../../../workbench/references/use-inspector-stack";
import { useThemeHost } from "../../ExtensionHost";
import type { ArtifactRendererProps } from "../../visual-renderers";

const BRIDGE_PROTOCOL = 1;
const MAX_BRIDGE_MESSAGE_BYTES = 64 * 1024;
const ALLOWED_LAYOUTS = new Set(["inline", "panel", "canvas"]);

type SandboxRequest = {
  protocol: number;
  type: "state.update" | "action.invoke" | "layout.request";
  requestId: string;
  payload: unknown;
};

type SandboxLifecycleMessage = {
  protocol: number;
  type: "ui.ready" | "ui.error";
  message?: string;
};

export function GeneratedReactArtifactViewer({
  detail,
  fallback,
}: ArtifactRendererProps) {
  const runtime = useQuery({
    queryKey: queryKeys.generatedUiRuntime(detail.artifact.artifact_id),
    queryFn: () => coreApi.generatedUiRuntime(detail.artifact.artifact_id),
  });
  const bundleHash = runtime.data?.build.bundle_blob_hash;
  const bundle = useQuery({
    queryKey: ["generated-ui-bundle", bundleHash],
    queryFn: () => coreApi.generatedUiBundle(bundleHash ?? ""),
    enabled: runtime.data?.build.state === "ready" && Boolean(bundleHash),
    staleTime: Number.POSITIVE_INFINITY,
  });

  if (runtime.isLoading || bundle.isLoading) {
    return <GeneratedUiStatus label="Preparing sandboxed UI…" />;
  }
  if (!runtime.data || runtime.data.build.state !== "ready" || !bundle.data) {
    return (
      <GeneratedUiFailure
        runtime={runtime.data}
        artifactId={detail.artifact.artifact_id}
        fallback={fallback}
        message={runtime.error ?? bundle.error}
      />
    );
  }
  const payload = generatedUiArtifactPayloadSchema.safeParse(
    runtime.data.artifact.version.payload,
  );
  if (!payload.success) {
    return (
      <GeneratedUiFailure
        runtime={runtime.data}
        artifactId={detail.artifact.artifact_id}
        fallback={fallback}
        message={new Error("Generated UI payload is invalid")}
      />
    );
  }
  return (
    <div className="space-y-2">
      <GeneratedUiSandbox
        runtime={runtime.data}
        payload={payload.data}
        bundle={bundle.data}
        fallback={fallback}
      />
      <GeneratedUiSourceDisclosure artifactId={detail.artifact.artifact_id} />
    </div>
  );
}

function GeneratedUiSandbox({
  runtime,
  payload,
  bundle,
  fallback,
}: {
  runtime: GeneratedUiRuntimeDetail;
  payload: GeneratedUiArtifactPayload;
  bundle: string;
  fallback: ReactNode;
}) {
  const queryClient = useQueryClient();
  const openReference = useOpenReference();
  const { effectiveTheme } = useThemeHost();
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const portRef = useRef<MessagePort | null>(null);
  const runtimeRef = useRef(runtime);
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [layout, setLayout] = useState(() => payload.layout_intent);
  runtimeRef.current = runtime;
  const source = useMemo(
    () => buildSandboxDocument(bundle, effectiveTheme.semantic_tokens),
    [bundle, effectiveTheme.semantic_tokens],
  );

  const sendSnapshot = useCallback(
    (port: MessagePort, next: GeneratedUiRuntimeDetail) => {
      port.postMessage({
        protocol: BRIDGE_PROTOCOL,
        type: "host.snapshot",
        payload: sandboxSnapshot(next, payload, layout),
      });
    },
    [layout],
  );

  const handleRequest = useCallback(
    async (port: MessagePort, message: SandboxRequest) => {
      const current = runtimeRef.current;
      try {
        assertBridgeRequest(message);
        let response: unknown;
        if (message.type === "state.update") {
          const messagePayload = asRecord(message.payload);
          if (
            typeof messagePayload.expected_revision !== "number" ||
            !Number.isSafeInteger(messagePayload.expected_revision)
          ) {
            throw new Error("State revision is invalid");
          }
          const state = await coreApi.updateGeneratedUiState(
            current.artifact.artifact.artifact_id,
            {
              expected_revision: messagePayload.expected_revision,
              values: messagePayload.values,
            },
          );
          const next = { ...current, state };
          runtimeRef.current = next;
          queryClient.setQueryData(
            queryKeys.generatedUiRuntime(current.artifact.artifact.artifact_id),
            next,
          );
          response = state;
          sendSnapshot(port, next);
        } else if (message.type === "action.invoke") {
          const messagePayload = asRecord(message.payload);
          if (typeof messagePayload.actionId !== "string") {
            throw new Error("Action identifier is invalid");
          }
          const definition = payload.actions.find(
            (action) => action.action_id === messagePayload.actionId,
          );
          if (!definition)
            throw new Error("Action is not declared by this artifact");
          const confirmed = definition.confirmation_required
            ? window.confirm(`Allow “${definition.label}”?`)
            : false;
          if (definition.confirmation_required && !confirmed) {
            throw new Error("Action was not confirmed");
          }
          const result = await coreApi.invokeGeneratedUiAction(
            current.artifact.artifact.artifact_id,
            definition.action_id,
            {
              artifact_version: current.artifact.version.version,
              idempotency_key: crypto.randomUUID(),
              input: messagePayload.input,
              confirmed,
            },
          );
          if (result.state) {
            const next = { ...current, state: result.state };
            runtimeRef.current = next;
            queryClient.setQueryData(
              queryKeys.generatedUiRuntime(
                current.artifact.artifact.artifact_id,
              ),
              next,
            );
            sendSnapshot(port, next);
          }
          if (definition.kind === "open_reference") {
            openReference(asRecord(result.result).reference as UpravaRef);
          }
          response = result.result;
        } else {
          const messagePayload = asRecord(message.payload);
          if (
            typeof messagePayload.layout !== "string" ||
            !ALLOWED_LAYOUTS.has(messagePayload.layout) ||
            !payload.granted_capabilities.includes("request_layout_change")
          ) {
            throw new Error("Layout request is not allowed");
          }
          setLayout(messagePayload.layout as "inline" | "panel" | "canvas");
          response = { layout: messagePayload.layout };
        }
        port.postMessage({
          protocol: BRIDGE_PROTOCOL,
          type: "host.response",
          requestId: message.requestId,
          ok: true,
          payload: response,
        });
      } catch (requestError) {
        port.postMessage({
          protocol: BRIDGE_PROTOCOL,
          type: "host.response",
          requestId: message.requestId,
          ok: false,
          error:
            requestError instanceof Error
              ? requestError.message
              : "Generated UI request failed",
        });
      }
    },
    [openReference, payload, queryClient, sendSnapshot],
  );

  const initialize = useCallback(() => {
    const frame = iframeRef.current;
    if (!frame?.contentWindow) return;
    portRef.current?.close();
    setReady(false);
    setError(null);
    const channel = new MessageChannel();
    portRef.current = channel.port1;
    channel.port1.onmessage = (event: MessageEvent<unknown>) => {
      const message = event.data;
      if (isLifecycleMessage(message)) {
        if (message.type === "ui.ready") setReady(true);
        else setError(message.message || "Generated UI runtime failed");
        return;
      }
      if (isSandboxRequest(message)) {
        void handleRequest(channel.port1, message);
      }
    };
    channel.port1.onmessageerror = () =>
      setError("Sandbox message could not be decoded");
    channel.port1.start();
    frame.contentWindow.postMessage(
      {
        protocol: BRIDGE_PROTOCOL,
        type: "uprava.ui.init",
        payload: sandboxSnapshot(runtimeRef.current, payload, layout),
      },
      "*",
      [channel.port2],
    );
  }, [handleRequest, layout, payload]);

  useEffect(() => () => portRef.current?.close(), []);
  useEffect(() => {
    if (ready || error) return;
    const timeout = window.setTimeout(
      () => setError("Generated UI did not initialize in time"),
      5_000,
    );
    return () => window.clearTimeout(timeout);
  }, [error, ready, source]);
  useEffect(() => {
    const port = portRef.current;
    if (port) sendSnapshot(port, runtimeRef.current);
  }, [layout, sendSnapshot]);

  if (error) {
    return (
      <div className="space-y-2">
        <GeneratedUiStatus label={error} tone="risk" />
        {fallback}
      </div>
    );
  }
  return (
    <div className="border border-[var(--color-border)] bg-[var(--color-bg)]">
      <div className="flex items-center justify-between border-b border-[var(--color-border)] px-2 py-1 text-xs">
        <span>Sandboxed Generated React · {layout}</span>
        <span className="text-[var(--color-muted)]">
          {ready ? "ready" : "loading"}
        </span>
      </div>
      <iframe
        ref={iframeRef}
        title={runtime.artifact.artifact.title}
        srcDoc={source}
        sandbox="allow-scripts"
        referrerPolicy="no-referrer"
        allow=""
        className={frameClass(layout)}
        onLoad={initialize}
        onError={() => setError("Generated UI iframe failed to load")}
      />
    </div>
  );
}

function GeneratedUiStatus({
  label,
  tone = "muted",
}: {
  label: string;
  tone?: "muted" | "risk";
}) {
  return (
    <div
      className={`border-l p-2 text-sm ${tone === "risk" ? "border-[var(--color-risk)] text-[var(--color-risk)]" : "border-[var(--color-border)] text-[var(--color-muted)]"}`}
    >
      {label}
    </div>
  );
}

function GeneratedUiFailure({
  runtime,
  artifactId,
  fallback,
  message,
}: {
  runtime: GeneratedUiRuntimeDetail | undefined;
  artifactId: string;
  fallback: ReactNode;
  message: unknown;
}) {
  const diagnostics = runtime?.build.diagnostics ?? [];
  const payload = runtime
    ? generatedUiArtifactPayloadSchema.safeParse(
        runtime.artifact.version.payload,
      )
    : null;
  const snapshot = payload?.success ? payload.data.fallback_snapshot : null;
  return (
    <div className="space-y-2">
      <GeneratedUiStatus
        tone="risk"
        label={
          diagnostics[0]?.message ??
          (message instanceof Error
            ? message.message
            : "Generated UI is unavailable")
        }
      />
      {snapshot ? (
        <img
          src={snapshot}
          alt={`${runtime?.artifact.artifact.title ?? "Generated UI"} fallback snapshot`}
          className="max-h-[32rem] w-full object-contain"
        />
      ) : null}
      {fallback}
      <GeneratedUiSourceDisclosure artifactId={artifactId} />
    </div>
  );
}

function GeneratedUiSourceDisclosure({ artifactId }: { artifactId: string }) {
  const [visible, setVisible] = useState(false);
  const source = useQuery({
    queryKey: ["generated-ui-source", artifactId],
    queryFn: () => coreApi.generatedUiSource(artifactId),
    enabled: visible,
    staleTime: Number.POSITIVE_INFINITY,
  });
  return (
    <div className="border-l border-[var(--color-border)] pl-2 text-xs">
      <button
        type="button"
        className="text-[var(--color-muted)] underline underline-offset-2"
        onClick={() => setVisible((current) => !current)}
      >
        {visible ? "Hide generated source" : "Review generated source"}
      </button>
      {visible ? (
        source.data ? (
          <pre className="mt-2 max-h-80 overflow-auto whitespace-pre-wrap border border-[var(--color-border)] bg-[var(--color-bg)] p-2 text-[var(--color-ink)]">
            {source.data}
          </pre>
        ) : (
          <p className="mt-2 text-[var(--color-muted)]">
            {source.error instanceof Error
              ? source.error.message
              : "Loading generated source…"}
          </p>
        )
      ) : null}
    </div>
  );
}

function sandboxSnapshot(
  runtime: GeneratedUiRuntimeDetail,
  payload: GeneratedUiArtifactPayload,
  layout: "inline" | "panel" | "canvas",
) {
  return {
    artifact: {
      artifact_id: runtime.artifact.artifact.artifact_id,
      version: runtime.artifact.version.version,
      title: runtime.artifact.artifact.title,
    },
    dataModel: payload.data_model,
    state: runtime.state,
    layout,
    actions: payload.actions,
  };
}

function asRecord(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("Sandbox payload must be an object");
  }
  return value as Record<string, unknown>;
}

function assertBridgeRequest(message: SandboxRequest) {
  if (message.protocol !== BRIDGE_PROTOCOL || !message.requestId) {
    throw new Error("Sandbox bridge protocol is invalid");
  }
  const size = new TextEncoder().encode(JSON.stringify(message)).byteLength;
  if (size > MAX_BRIDGE_MESSAGE_BYTES) {
    throw new Error("Sandbox message exceeds the size limit");
  }
}

function isSandboxRequest(value: unknown): value is SandboxRequest {
  if (!value || typeof value !== "object") return false;
  const message = value as Partial<SandboxRequest>;
  return (
    message.protocol === BRIDGE_PROTOCOL &&
    typeof message.requestId === "string" &&
    (message.type === "state.update" ||
      message.type === "action.invoke" ||
      message.type === "layout.request")
  );
}

function isLifecycleMessage(value: unknown): value is SandboxLifecycleMessage {
  if (!value || typeof value !== "object") return false;
  const message = value as Partial<SandboxLifecycleMessage>;
  return (
    message.protocol === BRIDGE_PROTOCOL &&
    (message.type === "ui.ready" || message.type === "ui.error") &&
    (message.message === undefined || typeof message.message === "string")
  );
}

function frameClass(layout: "inline" | "panel" | "canvas") {
  if (layout === "inline") return "block h-64 w-full border-0";
  if (layout === "panel") return "block h-[32rem] w-full border-0";
  return "block h-[min(70vh,48rem)] w-full border-0";
}

export function buildSandboxDocument(
  bundle: string,
  tokens: Record<string, string>,
) {
  const nonce = crypto.randomUUID().replaceAll("-", "");
  const color = (name: string, fallback: string) => {
    const value = tokens[name];
    return value && /^#[0-9a-f]{3,8}$/i.test(value) ? value : fallback;
  };
  const safeBundle = bundle
    .replaceAll(/<\/script/gi, "<\\/script")
    .replaceAll("<!--", "<\\!--");
  return `<!doctype html>
<html><head><meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'nonce-${nonce}'; style-src 'unsafe-inline'; img-src data: blob:; connect-src 'none'; font-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'">
<meta name="viewport" content="width=device-width, initial-scale=1">
<style>
:root{color-scheme:light dark;--u-bg:${color("surface.background", "rgb(255 255 255)")};--u-raised:${color("surface.raised", "rgb(245 245 245)")};--u-muted:${color("content.muted", "rgb(90 90 90)")};--u-fg:${color("content.primary", "rgb(20 20 20)")};--u-border:${color("border.default", "rgb(190 190 190)")};--u-risk:${color("status.risk", "rgb(160 40 40)")};font-family:ui-sans-serif,system-ui,sans-serif;color:var(--u-fg);background:var(--u-bg)}*{box-sizing:border-box}body{margin:0;padding:16px}h1,h2,h3,p{margin:0}.uprava-stack{display:flex;flex-direction:column}.uprava-row{display:flex;align-items:center;flex-wrap:wrap;gap:8px}.uprava-gap-sm{gap:8px}.uprava-gap-md{gap:16px}.uprava-gap-lg{gap:24px}.uprava-section,.uprava-card{border:1px solid var(--u-border);padding:16px;background:var(--u-raised)}.uprava-muted{color:var(--u-muted)}.uprava-badge{display:inline-block;border:1px solid var(--u-border);padding:2px 6px}.uprava-badge-risk{color:var(--u-risk)}button,input,select{font:inherit;color:inherit;background:var(--u-bg);border:1px solid var(--u-border);padding:8px}button{cursor:pointer}.uprava-table-wrap{overflow:auto}table{width:100%;border-collapse:collapse}th,td{text-align:left;border-bottom:1px solid var(--u-border);padding:8px}
</style></head><body><div id="uprava-generated-root"></div><script nonce="${nonce}">${safeBundle}</script></body></html>`;
}
