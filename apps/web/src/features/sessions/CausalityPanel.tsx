import {
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { useState } from "react";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import { EVENT_KIND_VALUES } from "../../shared/protocol/literals";
import type {
  CausalityLinks,
  DeductionRecord,
  EventEnvelope,
  TraceStep,
  UpravaRef,
} from "../../shared/protocol/types";
import { Badge, type BadgeTone } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { LoadingState } from "../../shared/ui/system";
import { Textarea } from "../../shared/ui/textarea";
import { runWorkbenchCommand } from "../../workbench/commands/registry";
import { ReferenceActions } from "../../workbench/references/ReferenceActions";
import { refTitle } from "../../workbench/references/refs";
import { useOpenReference } from "../../workbench/references/use-inspector-stack";

export function CausalityPanel({
  sessionThreadId,
}: {
  sessionThreadId: string;
}) {
  const queryClient = useQueryClient();
  const openReference = useOpenReference();
  const sessionRef: UpravaRef = {
    kind: "session",
    session_thread_id: sessionThreadId,
  };
  const [question, setQuestion] = useState("");
  const [deductionId, setDeductionId] = useState<string | null>(null);
  const trace = useQuery({
    queryKey: queryKeys.sessionTrace(sessionThreadId),
    queryFn: () => coreApi.sessionTrace(sessionThreadId),
  });
  const createDeduction = useMutation({
    mutationFn: () =>
      coreApi.createDeduction(sessionThreadId, {
        scope_ref: sessionRef,
        question: question.trim() || null,
      }),
    onSuccess: (accepted) => {
      setDeductionId(accepted.deduction_id);
      void queryClient.invalidateQueries({
        queryKey: queryKeys.session(sessionThreadId),
      });
    },
  });
  const deduction = useQuery({
    queryKey: queryKeys.deduction(deductionId ?? ""),
    queryFn: () => coreApi.deduction(deductionId ?? ""),
    enabled: Boolean(deductionId),
    refetchInterval: (query) => {
      const state = query.state.data?.state;
      return state === "requested" || state === "running" ? 1_500 : false;
    },
  });
  const persist = useMutation({
    mutationFn: (id: string) => coreApi.persistDeduction(id),
    onSuccess: async () => {
      if (!deductionId) return;
      await queryClient.invalidateQueries({
        queryKey: queryKeys.deduction(deductionId),
      });
    },
  });
  const cancel = useMutation({
    mutationFn: (id: string) => coreApi.cancelDeduction(id),
    onSuccess: async () => {
      if (!deductionId) return;
      await queryClient.invalidateQueries({
        queryKey: queryKeys.deduction(deductionId),
      });
    },
  });

  const openRef = (reference: UpravaRef) => {
    void runWorkbenchCommand("reference.openInInspector", {
      reference,
      openReference,
    });
  };

  return (
    <section
      className="space-y-4 border-t border-[var(--color-border)] pt-4"
      aria-label="Session trace"
    >
      <header className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <div className="zarya-label">Session Trace</div>
          <p className="mt-1 text-sm text-[var(--color-muted)]">
            Detailed session history with causal links and raw evidence.
          </p>
        </div>
        {trace.data ? (
          <div className="flex items-center gap-2">
            <Badge tone={precisionTone(trace.data.precision)}>
              {trace.data.precision}
            </Badge>
            <span className="text-xs text-[var(--color-muted)]">
              {trace.data.raw_event_count} raw events
            </span>
          </div>
        ) : null}
      </header>

      {trace.isError ? (
        <ErrorNotice error={trace.error} title="Trace load failed" />
      ) : trace.data ? (
        <div className="space-y-2">
          {trace.data.steps.map((step) => (
            <TraceStepCard key={step.block_id} step={step} openRef={openRef} />
          ))}
          {trace.data.steps.length === 0 ? (
            <p className="text-sm text-[var(--color-muted)]">
              No trace steps are available yet. Raw events remain accessible
              below.
            </p>
          ) : null}
        </div>
      ) : (
        <LoadingState stage="Loading causality trace" />
      )}

      <details className="border-t border-[var(--color-border)] pt-3">
        <summary className="cursor-pointer text-sm font-bold">
          Explain this session with Deduction
          <span className="ml-2 font-normal text-[var(--color-muted)]">
            Isolated analysis
          </span>
        </summary>
        <div className="mt-4 grid gap-4 border-l-2 border-[var(--color-notice)] pl-4 xl:grid-cols-[minmax(0,1fr)_minmax(18rem,0.8fr)]">
          <div>
            <div className="zarya-label">Deduction</div>
            <p className="mt-1 text-sm">
              Scope: <strong>whole session</strong>
            </p>
            <Textarea
              className="mt-3"
              value={question}
              maxLength={2_000}
              placeholder="What caused this result? Leave empty for the default question."
              aria-label="Deduction question"
              onChange={(event) => setQuestion(event.target.value)}
            />
            <div className="mt-2 flex flex-wrap gap-2">
              <Button
                variant="primary"
                disabled={createDeduction.isPending}
                onClick={() => createDeduction.mutate()}
              >
                {createDeduction.isPending ? "Starting…" : "Run Deduction"}
              </Button>
              <ReferenceActions reference={sessionRef} showCopy={false} />
            </div>
            {createDeduction.isError ? (
              <div className="mt-3">
                <ErrorNotice
                  error={createDeduction.error}
                  title="Deduction could not start"
                />
              </div>
            ) : null}
          </div>
          <DeductionResult
            deduction={deduction.data}
            loading={Boolean(deductionId && !deduction.data)}
            error={deduction.error}
            persistPending={persist.isPending}
            cancelPending={cancel.isPending}
            onPersist={(id) => persist.mutate(id)}
            onCancel={(id) => cancel.mutate(id)}
            openRef={openRef}
          />
        </div>
      </details>

      <RawEventLog sessionThreadId={sessionThreadId} />
    </section>
  );
}

function TraceStepCard({
  step,
  openRef,
}: {
  step: TraceStep;
  openRef: (reference: UpravaRef) => void;
}) {
  return (
    <article className="border-l-2 border-[var(--color-border-strong)] p-3">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="font-bold">{step.title}</h3>
            <Badge tone={precisionTone(step.precision)}>{step.precision}</Badge>
          </div>
          <p className="mt-1 text-sm text-[var(--color-muted)]">
            {step.summary}
          </p>
        </div>
        <ReferenceActions reference={step.primary_ref} showCopy={false} />
      </div>
      <AspectLinks links={step} openRef={openRef} />
    </article>
  );
}

function AspectLinks({
  links,
  openRef,
}: {
  links: CausalityLinks;
  openRef: (reference: UpravaRef) => void;
}) {
  const aspects = [
    ["source", links.source_refs],
    ["evidence", links.evidence_refs],
    ["cause", links.cause_refs],
    ["result", links.result_refs],
    ["raw", links.raw_refs],
  ] as const;
  return (
    <div className="mt-3 flex flex-wrap gap-x-4 gap-y-2">
      {aspects
        .filter(([, refs]) => refs.length > 0)
        .map(([aspect, refs]) => (
          <div key={aspect} className="flex flex-wrap items-center gap-1">
            <span className="text-[10px] font-bold uppercase text-[var(--color-muted)]">
              {aspect}
            </span>
            {refs.map((reference, index) => (
              <button
                key={`${aspect}:${index}:${refTitle(reference)}`}
                type="button"
                className="max-w-48 truncate border border-[var(--color-border-strong)] px-1.5 py-0.5 text-xs hover:border-[var(--color-ink)]"
                title={refTitle(reference)}
                onClick={() => openRef(reference)}
              >
                {refTitle(reference)}
              </button>
            ))}
          </div>
        ))}
    </div>
  );
}

function DeductionResult({
  deduction,
  loading,
  error,
  persistPending,
  cancelPending,
  onPersist,
  onCancel,
  openRef,
}: {
  deduction?: DeductionRecord;
  loading: boolean;
  error: unknown;
  persistPending: boolean;
  cancelPending: boolean;
  onPersist: (deductionId: string) => void;
  onCancel: (deductionId: string) => void;
  openRef: (reference: UpravaRef) => void;
}) {
  if (error) return <ErrorNotice error={error} title="Deduction load failed" />;
  if (loading) return <LoadingState stage="Waiting for Deduction" />;
  if (!deduction) {
    return (
      <div className="text-sm text-[var(--color-muted)]">
        A Deduction is isolated from the live agent session and uses only the
        bounded evidence snapshot.
      </div>
    );
  }
  const block = deduction.block;
  return (
    <article className="min-w-0 border border-[var(--color-border)] p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <Badge tone={deductionStateTone(deduction.state)}>
          {deduction.state}
        </Badge>
        {block ? (
          <Badge tone={certaintyTone(block.certainty)}>{block.certainty}</Badge>
        ) : null}
      </div>
      {deduction.state === "requested" || deduction.state === "running" ? (
        <div className="mt-3">
          <Button
            variant="danger"
            disabled={cancelPending}
            onClick={() => onCancel(deduction.deduction_id)}
          >
            {cancelPending ? "Cancelling…" : "Cancel Deduction"}
          </Button>
        </div>
      ) : null}
      {block ? (
        <>
          <h3 className="mt-3 font-bold">{block.title}</h3>
          <p className="mt-1 text-sm">{block.conclusion}</p>
          <ol className="mt-3 space-y-2">
            {block.steps.map((step) => (
              <li
                key={step.step_id}
                className="border-l border-[var(--color-border-strong)] pl-2 text-sm"
              >
                <Badge tone={classificationTone(step.classification)}>
                  {step.classification}
                </Badge>
                <p className="mt-1">{step.summary}</p>
                <div className="mt-1 flex flex-wrap gap-1">
                  {step.support_refs.map((reference, index) => (
                    <button
                      key={`${step.step_id}:${index}`}
                      type="button"
                      className="truncate border border-[var(--color-border-strong)] px-1.5 py-0.5 text-xs hover:border-[var(--color-ink)]"
                      onClick={() => openRef(reference)}
                    >
                      {refTitle(reference)}
                    </button>
                  ))}
                </div>
              </li>
            ))}
          </ol>
          <DeductionLists deduction={deduction} />
          <div className="mt-3 flex flex-wrap items-center gap-2">
            <Button
              disabled={persistPending || Boolean(deduction.artifact_id)}
              onClick={() => onPersist(deduction.deduction_id)}
            >
              {deduction.artifact_id ? "Persisted" : "Persist narrative"}
            </Button>
            <span className="text-xs text-[var(--color-muted)]">
              {block.provenance.provider} · {block.provenance.schema_version}
            </span>
          </div>
        </>
      ) : (
        <p className="mt-3 text-sm text-[var(--color-muted)]">
          {deduction.error_message ?? "The structured result is still pending."}
        </p>
      )}
      {deduction.raw_fallback ? (
        <details className="mt-3">
          <summary className="cursor-pointer text-xs font-bold">
            Raw fallback
          </summary>
          <pre className="mt-2 max-h-64 overflow-auto bg-[var(--color-bg-muted)] p-2 text-xs">
            {deduction.raw_fallback}
          </pre>
        </details>
      ) : null}
    </article>
  );
}

function DeductionLists({ deduction }: { deduction: DeductionRecord }) {
  const block = deduction.block;
  if (!block) return null;
  const lists = [
    ["Assumptions", block.assumptions],
    ["Unknowns", block.unknowns],
    ["Alternatives", block.alternatives],
  ] as const;
  return (
    <div className="mt-3 grid gap-2 sm:grid-cols-3">
      {lists.map(([label, values]) => (
        <div key={label}>
          <div className="text-xs font-bold">{label}</div>
          {values.length > 0 ? (
            <ul className="mt-1 list-disc pl-4 text-xs text-[var(--color-muted)]">
              {values.map((value, index) => (
                <li key={`${label}:${index}`}>{value}</li>
              ))}
            </ul>
          ) : (
            <span className="text-xs text-[var(--color-muted)]">
              None stated
            </span>
          )}
        </div>
      ))}
    </div>
  );
}

function RawEventLog({ sessionThreadId }: { sessionThreadId: string }) {
  const [kind, setKind] = useState("");
  const events = useInfiniteQuery({
    queryKey: queryKeys.eventLog(sessionThreadId, kind),
    queryFn: ({ pageParam }) =>
      coreApi.events({
        sessionThreadId,
        kind: kind || undefined,
        cursor: pageParam,
        limit: 50,
      }),
    initialPageParam: undefined as string | undefined,
    getNextPageParam: (page) => page.next_cursor ?? undefined,
  });
  const rows = events.data?.pages.flatMap((page) => page.events) ?? [];
  return (
    <details className="border-t border-[var(--color-border)] pt-3">
      <summary className="cursor-pointer text-sm font-bold">
        Raw event log
        <span className="ml-2 font-normal text-[var(--color-muted)]">
          {rows.length} loaded
        </span>
      </summary>
      <div className="mt-3 flex flex-wrap items-end gap-2">
        <label className="text-xs font-bold">
          Event kind
          <select
            className="ml-2 border border-[var(--color-muted)] bg-[var(--color-bg)] px-2 py-1 font-normal"
            value={kind}
            onChange={(event) => setKind(event.target.value)}
          >
            <option value="">All</option>
            {EVENT_KIND_VALUES.map((value) => (
              <option key={value} value={value}>
                {value}
              </option>
            ))}
          </select>
        </label>
      </div>
      {events.isError ? (
        <div className="mt-3">
          <ErrorNotice error={events.error} title="Event log load failed" />
        </div>
      ) : (
        <div className="mt-3 space-y-1">
          {rows.map((event) => (
            <RawEventRow key={event.event_id} event={event} />
          ))}
          {events.hasNextPage ? (
            <Button
              variant="ghost"
              disabled={events.isFetchingNextPage}
              onClick={() => events.fetchNextPage()}
            >
              {events.isFetchingNextPage ? "Loading…" : "Load older events"}
            </Button>
          ) : null}
        </div>
      )}
    </details>
  );
}

function RawEventRow({ event }: { event: EventEnvelope }) {
  const reference: UpravaRef = {
    kind: "event",
    event_id: event.event_id,
    scope_ref: event.scope_ref,
    seq: event.seq,
  };
  return (
    <details className="border-l border-[var(--color-border-strong)] py-1 pl-2 text-xs">
      <summary className="cursor-pointer">
        <strong>{event.kind}</strong> · seq {event.seq} · {event.happened_at}
      </summary>
      <div className="mt-2 flex justify-end">
        <ReferenceActions reference={reference} showCopy={false} />
      </div>
      <pre className="mt-1 max-h-72 overflow-auto bg-[var(--color-bg-muted)] p-2">
        {safeJson(event)}
      </pre>
    </details>
  );
}

function safeJson(value: unknown) {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return "[unserializable]";
  }
}

function precisionTone(value: string): BadgeTone {
  if (value === "exact") return "good";
  if (value === "coarse") return "info";
  return "warn";
}

function deductionStateTone(value: DeductionRecord["state"]): BadgeTone {
  if (value === "completed") return "good";
  if (value === "failed" || value === "invalid") return "bad";
  if (value === "running") return "info";
  return "warn";
}

function certaintyTone(value: string): BadgeTone {
  if (value === "high") return "good";
  if (value === "medium") return "info";
  return "warn";
}

function classificationTone(value: string): BadgeTone {
  if (value === "observed") return "good";
  if (value === "inference") return "info";
  if (value === "assumption") return "warn";
  return "neutral";
}
