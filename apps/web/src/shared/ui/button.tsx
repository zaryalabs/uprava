import { ButtonHTMLAttributes } from "react";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "ghost" | "danger";
};

const variants = {
  primary: "bg-[#2f7d6d] text-white hover:bg-[#27695d]",
  secondary:
    "border border-[#bfc8bc] bg-[#fbfcf8] text-[#17211c] hover:bg-[#edf1e9]",
  ghost: "text-[#27362f] hover:bg-[#e7ebe3]",
  danger: "bg-[#a83f3a] text-white hover:bg-[#87332f]",
};

export function Button({
  className = "",
  variant = "secondary",
  ...props
}: ButtonProps) {
  return (
    <button
      className={`inline-flex h-9 items-center justify-center gap-2 rounded-md px-3 text-sm font-medium transition disabled:cursor-not-allowed disabled:opacity-45 ${variants[variant]} ${className}`}
      {...props}
    />
  );
}
