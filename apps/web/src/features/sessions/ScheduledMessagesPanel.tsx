import { CalendarClock, RotateCcw, Send, X } from "lucide-react";
import { useMemo, useState } from "react";

import { coreApi } from "../../shared/api/http-client";
import type { ScheduledSessionMessage } from "../../shared/protocol/types";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { Textarea } from "../../shared/ui/textarea";

type Props = {
  sessionThreadId: string;
  messages: ScheduledSessionMessage[];
  onChanged: () => Promise<void> | void;
};

export function ScheduledMessagesPanel({
  sessionThreadId,
  messages,
  onChanged,
}: Props) {
  const [content, setContent] = useState("");
  const [dueAt, setDueAt] = useState(defaultDueAt());
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<unknown>();
  const timezone = useMemo(
    () => Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC",
    [],
  );

  const create = async () => {
    if (!content.trim() || !dueAt) return;
    setPending(true);
    setError(undefined);
    try {
      await coreApi.createScheduledMessage(sessionThreadId, {
        content: content.trim(),
        due_at: new Date(dueAt).toISOString(),
        timezone,
      });
      setContent("");
      setDueAt(defaultDueAt());
      await onChanged();
    } catch (nextError) {
      setError(nextError);
    } finally {
      setPending(false);
    }
  };

  const action = async (
    message: ScheduledSessionMessage,
    kind: "cancel" | "send" | "retry",
  ) => {
    setPending(true);
    setError(undefined);
    try {
      if (kind === "cancel") {
        await coreApi.cancelScheduledMessage(
          sessionThreadId,
          message.scheduled_message_id,
        );
      } else if (kind === "send") {
        await coreApi.sendScheduledMessageNow(
          sessionThreadId,
          message.scheduled_message_id,
        );
      } else {
        await coreApi.retryScheduledMessage(
          sessionThreadId,
          message.scheduled_message_id,
        );
      }
      await onChanged();
    } catch (nextError) {
      setError(nextError);
    } finally {
      setPending(false);
    }
  };

  return (
    <section
      className="border-t border-black/10 pt-4"
      aria-label="Delayed messages"
    >
      <div className="mb-2 flex items-center gap-2 text-sm font-bold">
        <CalendarClock size={16} aria-hidden="true" />
        Delayed Message
      </div>
      <p className="mb-3 text-xs text-[var(--color-muted)]">
        One future turn. Core stores it and rechecks session and runtime safety
        when it is due. Timezone: {timezone}.
      </p>
      {error ? <ErrorNotice error={error} title="Schedule failed" /> : null}
      <div className="grid gap-2 md:grid-cols-[minmax(0,1fr)_auto_auto]">
        <Textarea
          aria-label="Delayed turn content"
          value={content}
          onChange={(event) => setContent(event.target.value)}
          placeholder="Prepare a future turn"
          disabled={pending}
        />
        <input
          aria-label="Delayed turn time"
          type="datetime-local"
          value={dueAt}
          onChange={(event) => setDueAt(event.target.value)}
          disabled={pending}
          className="h-10 border border-[var(--color-muted)] bg-[var(--color-bg)] px-2 text-sm"
        />
        <Button
          type="button"
          variant="secondary"
          disabled={pending || !content.trim() || !dueAt}
          onClick={() => void create()}
        >
          <CalendarClock size={15} aria-hidden="true" />
          Schedule
        </Button>
      </div>
      {messages.length ? (
        <ul className="mt-3 space-y-2">
          {messages.map((message) => (
            <ScheduledMessageRow
              key={message.scheduled_message_id}
              message={message}
              pending={pending}
              onAction={action}
              onChanged={onChanged}
              onError={setError}
              sessionThreadId={sessionThreadId}
            />
          ))}
        </ul>
      ) : null}
    </section>
  );
}

function ScheduledMessageRow({
  message,
  pending,
  onAction,
  onChanged,
  onError,
  sessionThreadId,
}: {
  message: ScheduledSessionMessage;
  pending: boolean;
  onAction: (
    message: ScheduledSessionMessage,
    kind: "cancel" | "send" | "retry",
  ) => Promise<void>;
  onChanged: () => Promise<void> | void;
  onError: (error: unknown) => void;
  sessionThreadId: string;
}) {
  const [editing, setEditing] = useState(false);
  const [content, setContent] = useState(message.content);
  const [dueAt, setDueAt] = useState(toLocalDateTime(message.due_at));

  const reschedule = async () => {
    try {
      await coreApi.updateScheduledMessage(
        sessionThreadId,
        message.scheduled_message_id,
        {
          content: content.trim(),
          due_at: new Date(dueAt).toISOString(),
          timezone: message.timezone,
        },
      );
      setEditing(false);
      await onChanged();
    } catch (error) {
      onError(error);
    }
  };

  return (
    <li className="border border-black/10 bg-[var(--color-bg-muted)] p-3 text-sm">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span className="font-medium">{message.state}</span>
        <time
          className="text-xs text-[var(--color-muted)]"
          dateTime={message.due_at}
        >
          {new Date(message.due_at).toLocaleString()} · {message.timezone}
        </time>
      </div>
      {editing ? (
        <div className="mt-2 grid gap-2 md:grid-cols-[minmax(0,1fr)_auto_auto]">
          <Textarea
            aria-label="Rescheduled turn content"
            value={content}
            onChange={(event) => setContent(event.target.value)}
          />
          <input
            aria-label="Rescheduled turn time"
            type="datetime-local"
            value={dueAt}
            onChange={(event) => setDueAt(event.target.value)}
            className="h-10 border border-[var(--color-muted)] bg-[var(--color-bg)] px-2 text-sm"
          />
          <Button
            type="button"
            variant="secondary"
            disabled={pending || !content.trim() || !dueAt}
            onClick={() => void reschedule()}
          >
            Save
          </Button>
        </div>
      ) : (
        <p className="mt-1 whitespace-pre-wrap">{message.content}</p>
      )}
      {message.failure ? (
        <p className="mt-2 text-xs text-[var(--color-danger)]">
          {message.failure.code}: {message.failure.message}
        </p>
      ) : null}
      <div className="mt-2 flex flex-wrap gap-2">
        {message.state === "scheduled" ? (
          <>
            <Button
              type="button"
              variant="secondary"
              disabled={pending}
              onClick={() => void onAction(message, "send")}
            >
              <Send size={14} aria-hidden="true" /> Send now
            </Button>
            <Button
              type="button"
              variant="secondary"
              disabled={pending}
              onClick={() => setEditing((value) => !value)}
            >
              <RotateCcw size={14} aria-hidden="true" /> Reschedule
            </Button>
            <Button
              type="button"
              variant="danger"
              disabled={pending}
              onClick={() => void onAction(message, "cancel")}
            >
              <X size={14} aria-hidden="true" /> Cancel
            </Button>
          </>
        ) : null}
        {message.state === "failed" ? (
          <>
            <Button
              type="button"
              variant="secondary"
              disabled={pending}
              onClick={() => void onAction(message, "retry")}
            >
              <RotateCcw size={14} aria-hidden="true" /> Retry
            </Button>
            <Button
              type="button"
              variant="secondary"
              disabled={pending}
              onClick={() => setEditing((value) => !value)}
            >
              <CalendarClock size={14} aria-hidden="true" /> Reschedule
            </Button>
          </>
        ) : null}
      </div>
    </li>
  );
}

function defaultDueAt() {
  const value = new Date(Date.now() + 10 * 60 * 1000);
  value.setSeconds(0, 0);
  return toLocalDateTime(value.toISOString());
}

function toLocalDateTime(value: string) {
  const date = new Date(value);
  date.setMinutes(date.getMinutes() - date.getTimezoneOffset());
  return date.toISOString().slice(0, 16);
}
