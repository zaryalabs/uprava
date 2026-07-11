import type { PropsWithChildren } from "react";

type BadgeProps = PropsWithChildren<{
  tone?: "neutral" | "good" | "warn" | "bad" | "info";
}>;

const tones = {
  neutral: "border-[var(--color-muted)] text-[var(--color-muted)]",
  good: "border-[var(--color-ink)] text-[var(--color-ink)]",
  warn: "border-dashed border-[var(--color-muted)] text-[var(--color-ink)]",
  bad: "border-[var(--color-risk)] text-[var(--color-risk)]",
  info: "border-[var(--color-notice)] text-[var(--color-notice)]",
};

export function Badge({ children, tone = "neutral" }: BadgeProps) {
  return (
    <span
      className={`inline-flex min-h-6 items-center gap-1.5 border bg-[var(--color-bg)] px-2 py-0.5 text-xs font-medium ${tones[tone]}`}
    >
      {children}
    </span>
  );
}
