import { TextareaHTMLAttributes } from "react";

export function Textarea({
  className = "",
  ...props
}: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      className={`min-h-24 w-full resize-y rounded-md border border-[#bfc8bc] bg-white px-3 py-2 text-sm text-[#17211c] shadow-sm ${className}`}
      {...props}
    />
  );
}
