import { useId, useState, useEffect, type ReactNode } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useParams } from "react-router-dom";

import { coreApi } from "../../../shared/api/http-client";
import { queryKeys } from "../../../shared/api/query-keys";
import type { ArtifactDetail } from "../../../shared/protocol/types";
import { Button } from "../../../shared/ui/button";
import { ReferenceActions } from "../../../workbench/references/ReferenceActions";
import type {
  ArtifactRendererProps,
  InlineRendererProps,
} from "../../visual-renderers";

const MAX_DIAGRAM_SOURCE_CHARS = 32_000;
let mermaidInitialization: Promise<typeof import("mermaid")> | null = null;

export function DiagramRenderer(props: InlineRendererProps) {
  const { sessionThreadId } = useParams();
  const queryClient = useQueryClient();
  const pin = useMutation({
    mutationFn: async () => {
      if (!sessionThreadId) throw new Error("Session scope is unavailable");
      return coreApi.createArtifact({
        artifact_type: "uprava.diagram",
        title: `${diagramLabel(props.languageId)} diagram`,
        scope_ref: { kind: "session", session_thread_id: sessionThreadId },
        schema_version: 1,
        payload: { language: props.languageId, source: props.source },
        fallback_text: props.source,
        source_version: null,
        source_refs: [props.sourceRef],
        evidence_refs: [],
        cause_refs: [],
        trace_refs: [],
        provenance: {
          kind: "visual_promotion",
          renderer_id: "uprava.diagrams.markdown",
        },
      });
    },
    onSuccess: async (detail) => {
      await queryClient.invalidateQueries({
        queryKey: queryKeys.artifact(detail.artifact.artifact_id),
      });
      await queryClient.invalidateQueries({ queryKey: ["artifacts"] });
    },
  });

  return (
    <DiagramFrame
      source={props.source}
      languageId={props.languageId}
      fallback={props.fallback}
      actions={
        sessionThreadId ? (
          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="secondary"
              disabled={pin.isPending || pin.isSuccess}
              onClick={() => pin.mutate()}
            >
              {pin.isSuccess
                ? "Pinned"
                : pin.isPending
                  ? "Pinning…"
                  : "Pin artifact"}
            </Button>
            {pin.data ? (
              <ReferenceActions
                reference={{
                  kind: "artifact",
                  artifact_id: pin.data.artifact.artifact_id,
                }}
                showCopy={false}
              />
            ) : null}
            {pin.error ? (
              <span className="text-[var(--color-risk)]">
                {pin.error instanceof Error
                  ? pin.error.message
                  : "Artifact could not be pinned"}
              </span>
            ) : null}
          </div>
        ) : null
      }
    />
  );
}

export function DiagramArtifactViewer({
  detail,
  fallback,
}: ArtifactRendererProps) {
  const payload = diagramPayload(detail);
  if (!payload) return fallback;
  return (
    <DiagramFrame
      source={payload.source}
      languageId={payload.language}
      fallback={fallback}
      actions={
        <div className="font-mono text-xs text-[var(--color-muted)]">
          version {detail.version.version}
        </div>
      }
    />
  );
}

function DiagramFrame({
  source,
  languageId,
  fallback,
  actions,
}: {
  source: string;
  languageId: string;
  fallback: ReactNode;
  actions?: ReactNode;
}) {
  const instanceId = useId().replaceAll(":", "-");
  const [state, setState] = useState<
    | { kind: "loading" }
    | { kind: "ready"; dataUrl: string }
    | { kind: "error"; message: string }
  >({ kind: "loading" });

  useEffect(() => {
    let active = true;
    const render = async () => {
      try {
        const mermaid = await loadMermaid();
        const renderSource = normalizeDiagramSource(languageId, source);
        const result = await mermaid.default.render(
          `uprava-diagram-${instanceId}`,
          renderSource,
        );
        if (active) {
          setState({
            kind: "ready",
            dataUrl: `data:image/svg+xml;charset=utf-8,${encodeURIComponent(result.svg)}`,
          });
        }
      } catch (error) {
        if (active) {
          setState({
            kind: "error",
            message:
              error instanceof Error ? error.message : "Diagram render failed",
          });
        }
      }
    };
    if (source.length > MAX_DIAGRAM_SOURCE_CHARS) {
      setState({
        kind: "error",
        message: "Diagram source exceeds the bounded limit",
      });
      return () => undefined;
    }
    void render();
    return () => {
      active = false;
    };
  }, [instanceId, languageId, source]);

  if (state.kind === "loading") {
    return (
      <figure className="border-l border-[var(--color-muted)] bg-[var(--color-bg-muted)] p-3">
        <div className="text-xs text-[var(--color-muted)]">
          Rendering diagram…
        </div>
      </figure>
    );
  }
  if (state.kind === "error") {
    return (
      <figure className="border-l border-[var(--color-notice)] bg-[var(--color-notice-soft)] p-3">
        <figcaption className="mb-2 text-xs text-[var(--color-notice)]">
          {state.message}
        </figcaption>
        {fallback}
      </figure>
    );
  }
  return (
    <figure className="space-y-2 border-l border-[var(--color-border-strong)] bg-[var(--color-bg-raised)] p-3">
      <img
        className="max-h-[34rem] max-w-full"
        src={state.dataUrl}
        alt={`${diagramLabel(languageId)} diagram preview`}
      />
      <figcaption className="flex flex-wrap items-center justify-between gap-2 text-xs text-[var(--color-muted)]">
        <span>{diagramLabel(languageId)} · source-backed preview</span>
        {actions}
      </figcaption>
    </figure>
  );
}

async function loadMermaid() {
  mermaidInitialization ??= import("mermaid").then((module) => {
    module.default.initialize({
      startOnLoad: false,
      securityLevel: "strict",
      htmlLabels: false,
      maxTextSize: MAX_DIAGRAM_SOURCE_CHARS,
      suppressErrorRendering: true,
    });
    return module;
  });
  return mermaidInitialization;
}

export function normalizeDiagramSource(languageId: string, source: string) {
  if (/%%\{|<script|javascript:|!include|!import/i.test(source)) {
    throw new Error("Diagram source contains a disabled directive");
  }
  if (languageId === "mermaid") return source;
  if (languageId === "plantuml" || languageId === "puml") {
    return plantUmlToMermaid(source);
  }
  throw new Error(`Unsupported diagram language: ${languageId}`);
}

function plantUmlToMermaid(source: string) {
  const lines = source
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(
      (line) =>
        line.length > 0 &&
        line !== "@startuml" &&
        line !== "@enduml" &&
        !line.startsWith("'"),
    );
  const sequenceLines = lines.flatMap((line) => {
    const participant =
      /^(actor|participant|boundary|control|entity|database)\s+([\w.-]+)(?:\s+as\s+([\w.-]+))?$/i.exec(
        line,
      );
    if (participant) {
      const name = participant[3] ?? participant[2];
      return [`participant ${name}`];
    }
    const message = /^([\w.-]+)\s*(-+>>?|-+>)\s*([\w.-]+)\s*:\s*(.+)$/.exec(
      line,
    );
    return message
      ? [
          `${message[1]}${message[2].includes(">>") ? "-->>" : "->>"}${message[3]}: ${message[4]}`,
        ]
      : [];
  });
  if (sequenceLines.length > 0) {
    return ["sequenceDiagram", ...sequenceLines].join("\n");
  }
  const classLines = lines.filter((line) =>
    /^(?:abstract\s+)?class\s+[\w.-]+|^[\w.-]+\s+(?:<\|--|--\||\*--|o--|-->|<--)\s+[\w.-]+/.test(
      line,
    ),
  );
  if (classLines.length > 0) {
    return ["classDiagram", ...classLines].join("\n");
  }
  throw new Error(
    "PlantUML preview supports bounded sequence and class syntax",
  );
}

function diagramPayload(detail: ArtifactDetail) {
  const payload = detail.version.payload;
  if (!isRecord(payload)) return null;
  const language = payload.language;
  const source = payload.source;
  return typeof language === "string" && typeof source === "string"
    ? { language, source }
    : null;
}

function diagramLabel(languageId: string) {
  return languageId === "mermaid" ? "Mermaid" : "PlantUML";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
