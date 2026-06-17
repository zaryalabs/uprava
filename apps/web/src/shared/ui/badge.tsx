import { PropsWithChildren } from "react";

type BadgeProps = PropsWithChildren<{
  tone?: "neutral" | "good" | "warn" | "bad" | "info";
}>;

const tones = {
  neutral: "border-[#cad2c7] bg-[#f4f6f1] text-[#4d5a50]",
  good: "border-[#9ccdbd] bg-[#e4f4ef] text-[#1f6559]",
  warn: "border-[#d9c47d] bg-[#fff5ce] text-[#715b13]",
  bad: "border-[#dcaaa5] bg-[#fde5e2] text-[#88332f]",
  info: "border-[#a9c3d8] bg-[#e8f2fa] text-[#315d7d]",
};

export function Badge({ children, tone = "neutral" }: BadgeProps) {
  return (
    <span
      className={`inline-flex min-h-6 items-center gap-1.5 rounded-md border px-2 py-0.5 text-xs font-medium ${tones[tone]}`}
    >
      {children}
    </span>
  );
}
