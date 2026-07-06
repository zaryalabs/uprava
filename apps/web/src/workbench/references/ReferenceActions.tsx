import { useState } from "react";
import { Clipboard, PanelRightOpen } from "lucide-react";

import type { UpravaRef } from "../../shared/protocol/types";
import { Button } from "../../shared/ui/button";
import { runWorkbenchCommand } from "../commands/registry";
import { refTitle } from "./refs";
import { useOpenReference } from "./use-inspector-stack";

export function ReferenceActions({
  reference,
  showCopy = true,
  showInspect = true,
}: {
  reference: UpravaRef;
  showCopy?: boolean;
  showInspect?: boolean;
}) {
  const openReference = useOpenReference();
  const [copyState, setCopyState] = useState<"idle" | "copied" | "failed">(
    "idle",
  );
  const title = refTitle(reference);

  const inspect = () => {
    void runWorkbenchCommand("reference.openInInspector", {
      reference,
      openReference,
    });
  };
  const copy = () => {
    void runWorkbenchCommand("reference.copy", { reference })
      .then(() => setCopyState("copied"))
      .catch(() => setCopyState("failed"));
  };

  return (
    <span className="inline-flex items-center gap-1">
      {showInspect ? (
        <Button
          type="button"
          variant="ghost"
          className="h-7 w-7 px-0"
          aria-label={`Open ${title} in inspector`}
          title={`Open ${title} in inspector`}
          onClick={inspect}
        >
          <PanelRightOpen size={14} />
        </Button>
      ) : null}
      {showCopy ? (
        <Button
          type="button"
          variant="ghost"
          className="h-7 w-7 px-0"
          aria-label={`Copy ${title} reference`}
          title={
            copyState === "copied"
              ? "Reference copied"
              : copyState === "failed"
                ? "Clipboard unavailable"
                : `Copy ${title} reference`
          }
          onClick={copy}
        >
          <Clipboard size={14} />
        </Button>
      ) : null}
    </span>
  );
}
