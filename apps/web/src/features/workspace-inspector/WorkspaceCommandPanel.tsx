import { CheckCircle2, Play, SquareTerminal, XCircle } from "lucide-react";

import type { WorkspaceCommandRunResponse } from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";

export type RunCommandInput = {
  commandLine: string;
  intent: "command" | "check";
  label: string | null;
};

export function WorkspaceCommandPanel({
  commandText,
  isRunning,
  error,
  result,
  onCommandTextChange,
  onRun,
}: {
  commandText: string;
  isRunning: boolean;
  error: unknown;
  result: WorkspaceCommandRunResponse | null;
  onCommandTextChange: (value: string) => void;
  onRun: (input: RunCommandInput) => void;
}) {
  return (
    <section className="border border-[var(--color-muted)] bg-[var(--color-bg)]">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[var(--color-muted)] px-3 py-2">
        <div className="flex items-center gap-2 text-xs font-medium uppercase tracking-normal text-[var(--color-muted)]">
          <SquareTerminal size={15} />
          Command
        </div>
        <div className="flex flex-wrap gap-2">
          {[
            ["make l", "Local check"],
            ["make c", "Full check"],
          ].map(([commandLine, label]) => (
            <Button
              key={commandLine}
              variant="secondary"
              disabled={isRunning}
              onClick={() => onRun({ commandLine, intent: "check", label })}
            >
              <CheckCircle2 size={15} />
              {commandLine}
            </Button>
          ))}
        </div>
      </div>
      <div className="space-y-3 p-3">
        <div className="flex gap-2 max-sm:flex-col">
          <input
            className="h-9 min-w-0 flex-1 border border-[var(--color-muted)] bg-[var(--color-bg)] px-3 font-mono text-sm text-[var(--color-ink)]"
            value={commandText}
            onChange={(event) => onCommandTextChange(event.target.value)}
          />
          <Button
            variant="primary"
            disabled={isRunning}
            onClick={() =>
              onRun({
                commandLine: commandText,
                intent: "command",
                label: null,
              })
            }
          >
            <Play size={15} />
            {isRunning ? "Running" : "Run"}
          </Button>
        </div>
        {error ? <ErrorNotice error={error} title="Command failed" /> : null}
        {result ? <CommandResult result={result} /> : null}
      </div>
    </section>
  );
}

function CommandResult({ result }: { result: WorkspaceCommandRunResponse }) {
  return (
    <div className="space-y-2 border border-[var(--color-muted)] bg-[var(--color-bg-muted)] p-3">
      <div className="flex flex-wrap items-center gap-2">
        {result.success ? (
          <CheckCircle2 size={16} className="text-[var(--color-muted)]" />
        ) : (
          <XCircle size={16} className="text-[var(--color-risk)]" />
        )}
        <span className="font-mono text-sm">
          {formatCommandLine(result.command, result.args)}
        </span>
        <Badge tone={result.success ? "good" : "bad"}>
          exit {result.exit_code ?? "n/a"}
        </Badge>
        <Badge tone="neutral">{formatDuration(result.duration_ms)}</Badge>
      </div>
      {result.stdout ? (
        <OutputBlock
          title="stdout"
          content={result.stdout}
          truncated={result.stdout_truncated}
        />
      ) : null}
      {result.stderr ? (
        <OutputBlock
          title="stderr"
          content={result.stderr}
          truncated={result.stderr_truncated}
        />
      ) : null}
      {!result.stdout && !result.stderr ? (
        <div className="text-sm text-[var(--color-muted)]">No output</div>
      ) : null}
    </div>
  );
}

function OutputBlock({
  title,
  content,
  truncated,
}: {
  title: string;
  content: string;
  truncated: boolean;
}) {
  return (
    <div>
      <div className="mb-1 flex items-center gap-2 text-xs font-medium uppercase tracking-normal text-[var(--color-muted)]">
        <span>{title}</span>
        {truncated ? <Badge tone="warn">Truncated</Badge> : null}
      </div>
      <pre className="max-h-64 overflow-auto whitespace-pre-wrap bg-[var(--color-ink)] p-3 font-mono text-xs leading-5 text-[var(--color-bg)]">
        {content}
      </pre>
    </div>
  );
}

function formatCommandLine(command: string, args: string[]) {
  return [command, ...args].join(" ");
}

function formatDuration(durationMs: number) {
  return durationMs < 1000
    ? `${durationMs} ms`
    : `${(durationMs / 1000).toFixed(1)} s`;
}
