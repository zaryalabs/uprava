import type { TextareaHTMLAttributes } from "react";

export function Textarea({
  className = "",
  ...props
}: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      className={`min-h-24 w-full resize-y border border-[var(--color-muted)] bg-[var(--color-bg)] px-3 py-2 text-sm text-[var(--color-ink)] placeholder:text-[var(--color-muted)] disabled:cursor-not-allowed disabled:bg-[var(--color-bg-muted)] disabled:text-[var(--color-muted)] ${className}`}
      {...props}
    />
  );
}
