import { Send } from "lucide-react";
import { useState } from "react";

import { Button } from "../../shared/ui/button";
import { Textarea } from "../../shared/ui/textarea";

type Props = {
  pending: boolean;
  disabled?: boolean;
  onSend: (content: string) => void;
};

export function ChatComposer({ pending, disabled = false, onSend }: Props) {
  const [content, setContent] = useState("");

  return (
    <form
      className="rounded-md border border-[#d9ded4] bg-white p-3"
      onSubmit={(event) => {
        event.preventDefault();
        const trimmed = content.trim();
        if (disabled || !trimmed) return;
        onSend(trimmed);
        setContent("");
      }}
    >
      <Textarea
        value={content}
        onChange={(event) => setContent(event.target.value)}
        placeholder="Send a turn"
        disabled={disabled}
      />
      <div className="mt-2 flex justify-end">
        <Button
          variant="primary"
          disabled={disabled || pending || !content.trim()}
        >
          <Send size={15} />
          Send
        </Button>
      </div>
    </form>
  );
}
