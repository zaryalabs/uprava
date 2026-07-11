import type { HTMLAttributes, PropsWithChildren, ReactNode } from "react";

export function PageHeader({
  title,
  description,
  meta,
  actions,
}: {
  title: string;
  description?: string;
  meta?: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <header className="grid gap-4 pb-8 md:grid-cols-[minmax(0,1fr)_auto] md:items-end">
      <div className="min-w-0">
        {meta ? <div className="zarya-caption mb-2">{meta}</div> : null}
        <h1 className="text-2xl font-bold leading-[30px]">{title}</h1>
        {description ? (
          <p className="mt-2 max-w-3xl text-sm leading-5 text-[var(--color-muted)]">
            {description}
          </p>
        ) : null}
      </div>
      {actions ? <div className="flex flex-wrap gap-2">{actions}</div> : null}
    </header>
  );
}

export function Surface({
  className = "",
  ...props
}: HTMLAttributes<HTMLElement>) {
  return <section className={`zarya-section ${className}`} {...props} />;
}

export function FigureCaption({ children }: PropsWithChildren) {
  return <p className="zarya-caption mt-3">fig. {children}</p>;
}

export function EmptyState({
  title,
  detail,
}: {
  title: string;
  detail?: string;
}) {
  return (
    <div className="py-6 text-sm" role="status">
      <div className="font-medium">{title}</div>
      {detail ? (
        <div className="mt-1 text-[var(--color-muted)]">{detail}</div>
      ) : null}
    </div>
  );
}

export function LoadingState({ stage }: { stage: string }) {
  return (
    <div
      className="py-6 text-sm text-[var(--color-muted)]"
      role="status"
      aria-live="polite"
    >
      {stage}…
    </div>
  );
}

export function DisclosureControl({
  expanded,
  label,
  onClick,
}: {
  expanded: boolean;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-expanded={expanded}
      aria-label={`${expanded ? "Collapse" : "Expand"} ${label}`}
      className="grid h-7 w-7 shrink-0 place-items-center border border-[var(--color-muted)] bg-[var(--color-bg)] text-base leading-none hover:border-[var(--color-ink)] hover:bg-[var(--color-bg-muted)]"
      onClick={onClick}
    >
      <span aria-hidden="true">{expanded ? "−" : "+"}</span>
    </button>
  );
}
