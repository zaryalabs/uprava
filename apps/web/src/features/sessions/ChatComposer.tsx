import { Send } from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "../../shared/ui/button";
import { Textarea } from "../../shared/ui/textarea";

type Props = {
  pending: boolean;
  disabled?: boolean;
  onSend: (content: string) => Promise<void> | void;
};

export function ChatComposer({ pending, disabled = false, onSend }: Props) {
  const [content, setContent] = useState("");

  useEffect(() => {
    if (!content.trim()) return;
    const warnBeforeUnload = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", warnBeforeUnload);
    return () => window.removeEventListener("beforeunload", warnBeforeUnload);
  }, [content]);

  return (
    <form
      className="border-t border-black/10 pt-4"
      onSubmit={(event) => {
        event.preventDefault();
        const trimmed = content.trim();
        if (disabled || !trimmed) return;
        void Promise.resolve(onSend(trimmed))
          .then(() => {
            setContent("");
          })
          .catch(() => {});
      }}
    >
      <div className="mb-2 flex flex-wrap items-baseline justify-between gap-2">
        <label htmlFor="session-turn" className="text-sm font-bold">
          Next Agent Turn
        </label>
        <span className="text-xs text-[var(--color-muted)]">
          {disabled
            ? "Runtime cannot accept a turn in its current state."
            : "Draft stays until the turn is accepted."}
        </span>
      </div>
      <Textarea
        id="session-turn"
        name="session-turn"
        autoComplete="off"
        value={content}
        onChange={(event) => setContent(event.target.value)}
        placeholder="Send a turn"
        disabled={disabled}
      />
      <div className="mt-2 flex items-center justify-between gap-3">
        <span
          className="text-xs text-[var(--color-muted)]"
          role="status"
          aria-live="polite"
        >
          {pending
            ? "Sending turn…"
            : content.trim()
              ? "Draft not sent"
              : "Ready"}
        </span>
        <Button
          type="submit"
          variant="primary"
          disabled={disabled || pending || !content.trim()}
        >
          <Send size={15} aria-hidden="true" />
          {pending ? "Sending…" : "Send Turn"}
        </Button>
      </div>
    </form>
  );
}
