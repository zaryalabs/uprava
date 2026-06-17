import { useMutation, useQueryClient } from "@tanstack/react-query";
import { LogIn, LogOut, Pause, Play, RotateCcw, Square } from "lucide-react";

import { queryKeys } from "../../shared/api/query-keys";
import type {
  RuntimeSummary,
  SessionSummary,
} from "../../shared/protocol/types";
import { Button } from "../../shared/ui/button";
import {
  canRunCommand,
  runWorkbenchCommand,
  type WorkbenchCommandId,
  type WorkbenchCommandContext,
} from "../../workbench/commands/registry";

type Props = {
  session: SessionSummary;
  runtime: RuntimeSummary;
};

export function LifecycleControls({ session, runtime }: Props) {
  const queryClient = useQueryClient();
  const invalidate = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: queryKeys.inventory }),
      queryClient.invalidateQueries({
        queryKey: queryKeys.session(session.session_thread_id),
      }),
    ]);
  };
  const context = { session, runtime, afterSuccess: invalidate };
  const command = useMutation({
    mutationFn: (id: WorkbenchCommandId) => runWorkbenchCommand(id, context),
  });
  const controls = lifecycleControlStates(context, command.isPending);

  return (
    <div className="flex flex-wrap gap-2">
      {controls.map((control) => {
        const Icon = control.icon;
        return (
          <Button
            key={control.id}
            variant={control.variant}
            disabled={!control.enabled}
            onClick={() => command.mutate(control.id)}
          >
            <Icon size={15} />
            {control.label}
          </Button>
        );
      })}
    </div>
  );
}

export type LifecycleControlState = {
  id: Extract<
    WorkbenchCommandId,
    | "session.attach"
    | "session.detach"
    | "runtime.interrupt"
    | "runtime.stop"
    | "runtime.resume"
  >;
  label: string;
  enabled: boolean;
  icon: typeof LogIn;
  variant?: "primary" | "secondary" | "ghost" | "danger";
};

export function lifecycleControlStates(
  context: WorkbenchCommandContext,
  pending = false,
): LifecycleControlState[] {
  return [
    {
      id: "session.attach",
      label: "Attach",
      enabled: !pending && canRunCommand("session.attach", context),
      icon: LogIn,
    },
    {
      id: "session.detach",
      label: "Detach",
      enabled: !pending && canRunCommand("session.detach", context),
      icon: LogOut,
    },
    {
      id: "runtime.interrupt",
      label: "Interrupt",
      enabled: !pending && canRunCommand("runtime.interrupt", context),
      icon: Pause,
    },
    {
      id: "runtime.stop",
      label: "Stop",
      enabled: !pending && canRunCommand("runtime.stop", context),
      icon: Square,
      variant: "danger",
    },
    {
      id: "runtime.resume",
      label: "Resume",
      enabled: !pending && canRunCommand("runtime.resume", context),
      icon: context.runtime?.resume_supported ? RotateCcw : Play,
    },
  ];
}
