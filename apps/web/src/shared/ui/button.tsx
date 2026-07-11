import type { ButtonHTMLAttributes } from "react";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "ghost" | "danger";
};

const variants = {
  primary:
    "border border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-bg)] hover:opacity-80 active:opacity-65",
  secondary:
    "border border-[var(--color-muted)] bg-[var(--color-bg)] text-[var(--color-ink)] hover:border-[var(--color-ink)] hover:bg-[var(--color-bg-muted)]",
  ghost:
    "border border-transparent text-[var(--color-ink)] hover:border-[var(--color-muted)] hover:bg-[var(--color-bg-muted)]",
  danger:
    "border border-[var(--color-risk)] bg-[var(--color-risk)] text-[var(--color-bg)] hover:opacity-80 active:opacity-65",
};

export function Button({
  className = "",
  variant = "secondary",
  ...props
}: ButtonProps) {
  return (
    <button
      className={`inline-flex h-9 shrink-0 items-center justify-center gap-2 px-3 text-sm font-medium disabled:cursor-not-allowed disabled:border-[var(--color-muted)] disabled:bg-[var(--color-bg)] disabled:text-[var(--color-muted)] disabled:opacity-55 ${variants[variant]} ${className}`}
      {...props}
    />
  );
}
