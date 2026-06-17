import type { ComponentType, ReactNode } from "react";
import {
  AlertTriangle,
  Bot,
  CheckCircle2,
  CircleDot,
  HelpCircle,
  User,
} from "lucide-react";

import type { UiBlock } from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";

type BadgeTone = "neutral" | "good" | "warn" | "bad" | "info";

export type BlockRendererProps = {
  block: UiBlock;
  actions?: ReactNode;
};

export type BlockFallbackProps = BlockRendererProps;

export type BlockRendererRegistration = {
  type: string;
  supportedSchemaVersions: number[];
  allowedSurfaces: string[];
  render: ComponentType<BlockRendererProps>;
  fallback: ComponentType<BlockFallbackProps>;
};

const rendererRegistrations: BlockRendererRegistration[] = [
  register("core.user-message", MessageBlock),
  register("core.assistant-message", MessageBlock),
  register("core.provider-output-stream", EventBlock),
  register("core.runtime-event", EventBlock),
  register("core.workspace-validation", EventBlock),
  register("core.resource-snapshot", EventBlock),
  register("core.warning", WarningBlock),
  register("core.error", ErrorBlock),
  register("core.approval-request", ApprovalBlock),
  register("core.unknown", UnknownBlock),
];

const rendererByType = new Map(
  rendererRegistrations.map((registration) => [
    registration.type,
    registration,
  ]),
);

export function TimelineBlockRenderer({ block, actions }: BlockRendererProps) {
  const registration = rendererByType.get(block.type);
  if (
    !registration ||
    !registration.supportedSchemaVersions.includes(block.schema_version)
  ) {
    const Fallback = registration?.fallback ?? UnknownBlock;
    return <Fallback block={block} actions={actions} />;
  }
  const Renderer = registration.render;
  return <Renderer block={block} actions={actions} />;
}

export function getTimelineBlockRenderer(type: string) {
  return rendererByType.get(type);
}

export function registeredTimelineBlockTypes() {
  return rendererRegistrations.map((registration) => registration.type);
}

function register(
  type: string,
  render: ComponentType<BlockRendererProps>,
  fallback: ComponentType<BlockFallbackProps> = UnknownBlock,
): BlockRendererRegistration {
  return {
    type,
    supportedSchemaVersions: [1],
    allowedSurfaces: ["session.timeline"],
    render,
    fallback,
  };
}

function MessageBlock({ block, actions }: BlockRendererProps) {
  const data = blockData(block);
  const isAssistant = block.type === "core.assistant-message";
  const label = isAssistant ? "Assistant" : "User";
  const Icon = isAssistant ? Bot : User;

  return (
    <article
      className={
        isAssistant
          ? "rounded-md border border-[#d9ded4] bg-white p-3"
          : "rounded-md border border-[#c4d7cf] bg-[#edf7f3] p-3"
      }
    >
      <div
        className={
          isAssistant
            ? "mb-1 flex items-center gap-1.5 text-xs font-semibold uppercase tracking-normal text-[#667268]"
            : "mb-1 flex items-center gap-1.5 text-xs font-semibold uppercase tracking-normal text-[#2f7d6d]"
        }
      >
        <Icon size={14} />
        {label}
      </div>
      <p className="whitespace-pre-wrap break-words text-sm">
        {stringField(data, "content", block.fallback_text ?? "")}
      </p>
      <BlockActions actions={actions} />
    </article>
  );
}

function ApprovalBlock({ block, actions }: BlockRendererProps) {
  const data = blockData(block);

  return (
    <article className="rounded-md border border-[#d7cba3] bg-[#fffaf0] p-3">
      <div className="mb-2 flex items-center gap-2">
        <Badge tone="warn">
          <CircleDot size={13} />
          Approval
        </Badge>
        <span className="font-mono text-xs text-[#667268]">
          {stringField(data, "approvalId", "pending")}
        </span>
      </div>
      <p className="whitespace-pre-wrap break-words text-sm">
        {stringField(
          data,
          "prompt",
          block.fallback_text ?? "Approval requested",
        )}
      </p>
      <BlockActions actions={actions} />
    </article>
  );
}

function EventBlock({ block, actions }: BlockRendererProps) {
  const data = blockData(block);
  const eventKind = stringField(data, "eventKind", block.type);

  return (
    <article className="rounded-md border border-[#d7cba3] bg-[#fffaf0] p-3">
      <div className="mb-1 flex flex-wrap items-center gap-2">
        <Badge tone={eventTone(block.type)}>
          <CheckCircle2 size={13} />
          {eventKind}
        </Badge>
        <span className="font-mono text-xs text-[#667268]">
          seq {numberField(data, "seq", 0)}
        </span>
      </div>
      <div className="break-words text-sm text-[#536257]">
        {stringField(data, "summary", block.fallback_text ?? eventKind)}
      </div>
      <BlockActions actions={actions} />
    </article>
  );
}

function WarningBlock({ block, actions }: BlockRendererProps) {
  const data = blockData(block);

  return (
    <article className="rounded-md border border-[#d9c47d] bg-[#fff5ce] p-3">
      <div className="mb-1 flex items-center gap-2 text-xs font-semibold uppercase tracking-normal text-[#715b13]">
        <AlertTriangle size={14} />
        {stringField(data, "eventKind", "Warning")}
      </div>
      <div className="break-words text-sm text-[#536257]">
        {stringField(data, "summary", block.fallback_text ?? "Warning")}
      </div>
      <BlockActions actions={actions} />
    </article>
  );
}

function ErrorBlock({ block, actions }: BlockRendererProps) {
  const data = blockData(block);

  return (
    <article className="rounded-md border border-[#dcaaa5] bg-[#fde5e2] p-3">
      <div className="mb-1 flex items-center gap-2 text-xs font-semibold uppercase tracking-normal text-[#88332f]">
        <AlertTriangle size={14} />
        {stringField(data, "eventKind", "Error")}
      </div>
      <div className="break-words text-sm text-[#536257]">
        {stringField(data, "summary", block.fallback_text ?? "Runtime error")}
      </div>
      <BlockActions actions={actions} />
    </article>
  );
}

function UnknownBlock({ block, actions }: BlockFallbackProps) {
  return (
    <article className="rounded-md border border-[#d9ded4] bg-white p-3">
      <div className="mb-1 flex items-center gap-2 text-xs font-semibold uppercase tracking-normal text-[#667268]">
        <HelpCircle size={14} />
        Unknown
      </div>
      <div className="break-words text-sm">
        {block.fallback_text || block.type}
      </div>
      <BlockActions actions={actions} />
    </article>
  );
}

function BlockActions({ actions }: { actions?: ReactNode }) {
  return actions ? (
    <div className="mt-3 flex flex-wrap items-center gap-2">{actions}</div>
  ) : null;
}

function eventTone(type: string): BadgeTone {
  if (
    type === "core.workspace-validation" ||
    type === "core.resource-snapshot"
  ) {
    return "good";
  }
  if (type === "core.provider-output-stream") return "info";
  return "warn";
}

function blockData(block: UiBlock): Record<string, unknown> {
  return typeof block.data === "object" && block.data !== null
    ? (block.data as Record<string, unknown>)
    : {};
}

function stringField(
  data: Record<string, unknown>,
  field: string,
  fallback: string,
) {
  const value = data[field];
  return typeof value === "string" && value.length > 0 ? value : fallback;
}

function numberField(
  data: Record<string, unknown>,
  field: string,
  fallback: number,
) {
  const value = data[field];
  return typeof value === "number" ? value : fallback;
}
