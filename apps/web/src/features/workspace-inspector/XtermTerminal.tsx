import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { Terminal, type ITheme } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { XCircle } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { useThemeHost } from "../../plugins/ExtensionHost";
import { coreApi } from "../../shared/api/http-client";
import type {
  WorkspaceTerminalState,
  WorkspaceTerminalSummary,
} from "../../shared/protocol/types";
import { parseTerminalStreamFrame } from "../../shared/protocol/validators";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";

export function XtermTerminalPanel({
  placementId,
  terminal,
  onStatusChange,
}: {
  placementId: string;
  terminal: WorkspaceTerminalSummary;
  onStatusChange: () => void;
}) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const socketRef = useRef<WebSocket | null>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const themeColorsRef = useRef<Record<string, string>>({});
  const onStatusChangeRef = useRef(onStatusChange);
  const [state, setState] = useState(terminal.state);
  const [exitCode, setExitCode] = useState<number | null>(terminal.exit_code);
  const { effectiveTheme } = useThemeHost();
  themeColorsRef.current = effectiveTheme.terminal.colors;
  onStatusChangeRef.current = onStatusChange;

  useEffect(() => {
    setState(terminal.state);
    setExitCode(terminal.exit_code);
  }, [terminal.exit_code, terminal.state]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const term = new Terminal({
      cursorBlink: true,
      convertEol: true,
      scrollback: 2_000,
      fontFamily:
        "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
      fontSize: 12,
      lineHeight: 1.25,
      theme: xtermTheme(themeColorsRef.current),
    });
    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.loadAddon(new WebLinksAddon());
    term.open(container);
    terminalRef.current = term;
    fitAddon.fit();
    term.focus();

    const socket = new WebSocket(
      coreApi.workspaceTerminalStreamUrl(placementId, terminal.terminal_id),
    );
    socketRef.current = socket;
    const sendFrame = (frame: unknown) => {
      if (socket.readyState === WebSocket.OPEN) {
        socket.send(JSON.stringify(frame));
      }
    };
    const sendResize = () => {
      fitAddon.fit();
      sendFrame({ kind: "resize", cols: term.cols, rows: term.rows });
    };
    const dataDisposable = term.onData((data) => {
      sendFrame({ kind: "input", data });
    });
    socket.addEventListener("open", sendResize);
    socket.addEventListener("message", (event) => {
      const frame = parseTerminalStreamFrame(event.data);
      if (!frame) {
        setState("error");
        setExitCode(null);
        term.writeln("\r\nTerminal stream protocol validation failed");
        socket.close(1002, "protocol validation failed");
        return;
      }
      if (frame.kind === "output") term.write(frame.data);
      if (frame.kind === "status") {
        setState(frame.state);
        setExitCode(frame.exit_code);
        onStatusChangeRef.current();
      }
      if (frame.kind === "error") term.writeln(`\r\n${frame.message}`);
    });
    const resizeObserver =
      typeof ResizeObserver === "undefined"
        ? null
        : new ResizeObserver(sendResize);
    resizeObserver?.observe(container);
    return () => {
      resizeObserver?.disconnect();
      dataDisposable.dispose();
      socket.close();
      term.dispose();
      terminalRef.current = null;
      socketRef.current = null;
    };
  }, [placementId, terminal.terminal_id]);

  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.options.theme = xtermTheme(
        effectiveTheme.terminal.colors,
      );
    }
  }, [effectiveTheme.terminal.colors]);

  const closeTerminal = () => {
    if (socketRef.current?.readyState === WebSocket.OPEN) {
      socketRef.current.send(JSON.stringify({ kind: "close" }));
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden border border-[var(--color-border-strong)] bg-[var(--color-terminal-bg)]">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[var(--color-border)] bg-[var(--color-bg-raised)] px-3 py-2">
        <div className="flex min-w-0 items-center gap-2">
          <span className="truncate font-mono text-xs text-[var(--color-terminal-ink)]">
            {terminal.shell}
          </span>
          <Badge tone={terminalStateTone(state)}>
            {terminalStateLabel(state, exitCode)}
          </Badge>
        </div>
        <Button
          variant="secondary"
          disabled={state === "closed" || state === "exited"}
          onClick={closeTerminal}
        >
          <XCircle size={15} />
          Close
        </Button>
      </div>
      <div
        ref={containerRef}
        className="min-h-32 flex-1 p-2"
        role="region"
        aria-label={`Terminal ${terminal.shell}`}
      />
    </div>
  );
}

function xtermTheme(colors: Record<string, string>): ITheme {
  return {
    background: colors.background,
    foreground: colors.foreground,
    cursor: colors.cursor,
    selectionBackground: colors.selectionBackground,
    black: colors.black,
    red: colors.red,
    green: colors.green,
    yellow: colors.yellow,
    blue: colors.blue,
    magenta: colors.magenta,
    cyan: colors.cyan,
    white: colors.white,
  };
}

function terminalStateTone(state: WorkspaceTerminalState) {
  if (state === "running") return "good";
  if (state === "opening" || state === "detached") return "info";
  if (state === "error") return "bad";
  return "neutral";
}

function terminalStateLabel(
  state: WorkspaceTerminalState,
  exitCode: number | null,
) {
  return state === "exited"
    ? `Exited ${exitCode ?? "n/a"}`
    : state.charAt(0).toUpperCase() + state.slice(1);
}
