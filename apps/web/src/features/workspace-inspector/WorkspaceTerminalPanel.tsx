import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Plus, SquareTerminal } from "lucide-react";
import { lazy, Suspense, useEffect, useState } from "react";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type { WorkspaceTerminalSummary } from "../../shared/protocol/types";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";

const XtermTerminalPanel = lazy(() =>
  import("./XtermTerminal").then((module) => ({
    default: module.XtermTerminalPanel,
  })),
);

export function WorkspaceTerminalPanel({
  placementId,
}: {
  placementId: string;
}) {
  const queryClient = useQueryClient();
  const [activeTerminalId, setActiveTerminalId] = useState<string | null>(null);
  const terminals = useQuery({
    queryKey: queryKeys.workspaceTerminals(placementId),
    queryFn: () => coreApi.workspaceTerminals(placementId),
    enabled: Boolean(placementId),
  });
  const openTerminal = useMutation({
    mutationFn: () =>
      coreApi.openWorkspaceTerminal(placementId, {
        shell_profile: null,
        cols: 80,
        rows: 24,
      }),
    onSuccess: (response) => {
      setActiveTerminalId(response.terminal.terminal_id);
      void queryClient.invalidateQueries({
        queryKey: queryKeys.workspaceTerminals(placementId),
      });
    },
  });

  const terminalList = terminals.data?.terminals ?? [];
  const firstTerminalId = terminalList[0]?.terminal_id ?? null;
  const activeTerminal =
    terminalList.find(
      (terminal) => terminal.terminal_id === activeTerminalId,
    ) ??
    terminalList[0] ??
    null;

  useEffect(() => {
    if (!activeTerminalId && firstTerminalId)
      setActiveTerminalId(firstTerminalId);
  }, [activeTerminalId, firstTerminalId]);

  const refresh = () => {
    void queryClient.invalidateQueries({
      queryKey: queryKeys.workspaceTerminals(placementId),
    });
  };

  return (
    <section className="rounded-md border border-[#d9ded4] bg-white">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[#e0e5db] px-3 py-2">
        <div className="flex items-center gap-2 text-xs font-medium uppercase tracking-normal text-[#667268]">
          <SquareTerminal size={15} />
          Terminal
        </div>
        <Button
          variant="secondary"
          disabled={openTerminal.isPending}
          onClick={() => openTerminal.mutate()}
        >
          <Plus size={15} />
          {openTerminal.isPending ? "Opening" : "New"}
        </Button>
      </div>
      <div className="space-y-3 p-3">
        {terminals.error ? (
          <ErrorNotice error={terminals.error} title="Terminals unavailable" />
        ) : null}
        {openTerminal.error ? (
          <ErrorNotice error={openTerminal.error} title="Terminal failed" />
        ) : null}
        {terminalList.length > 0 ? (
          <div className="flex flex-wrap gap-2">
            {terminalList.map((terminal) => (
              <button
                key={terminal.terminal_id}
                type="button"
                className={`min-h-8 rounded-md border px-3 text-left font-mono text-xs ${
                  terminal.terminal_id === activeTerminal?.terminal_id
                    ? "border-[#1f6559] bg-[#e4ece1] text-[#173a2c]"
                    : "border-[#d9ded4] bg-[#fbfcf8] text-[#536257]"
                }`}
                onClick={() => setActiveTerminalId(terminal.terminal_id)}
              >
                {terminalLabel(terminal)}
              </button>
            ))}
          </div>
        ) : null}
        {activeTerminal ? (
          <Suspense fallback={<Fallback />}>
            <XtermTerminalPanel
              key={activeTerminal.terminal_id}
              placementId={placementId}
              terminal={activeTerminal}
              onStatusChange={refresh}
            />
          </Suspense>
        ) : (
          <div className="flex min-h-36 items-center justify-center text-sm text-[#536257]">
            No terminal open
          </div>
        )}
      </div>
    </section>
  );
}

function terminalLabel(terminal: WorkspaceTerminalSummary) {
  const title = terminal.title || terminal.shell || terminal.terminal_id;
  const state =
    terminal.state === "exited"
      ? `Exited ${terminal.exit_code ?? "n/a"}`
      : terminal.state.charAt(0).toUpperCase() + terminal.state.slice(1);
  return `${title} · ${state}`;
}

function Fallback() {
  return (
    <div className="flex min-h-24 items-center justify-center">
      Loading terminal
    </div>
  );
}
