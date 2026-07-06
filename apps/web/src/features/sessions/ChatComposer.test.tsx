import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ChatComposer } from "./ChatComposer";

describe("ChatComposer", () => {
  it("preserves the draft when send fails", async () => {
    const onSend = vi.fn(() => Promise.reject(new Error("send failed")));
    render(<ChatComposer pending={false} onSend={onSend} />);

    const textarea = screen.getByPlaceholderText(
      "Send a turn",
    ) as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: "keep this draft" } });
    fireEvent.click(screen.getByRole("button", { name: /Send/i }));

    await waitFor(() => expect(onSend).toHaveBeenCalledWith("keep this draft"));
    expect(textarea).toHaveValue("keep this draft");
  });

  it("clears the draft after send succeeds", async () => {
    const onSend = vi.fn(() => Promise.resolve());
    render(<ChatComposer pending={false} onSend={onSend} />);

    const textarea = screen.getByPlaceholderText(
      "Send a turn",
    ) as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: "send this draft" } });
    fireEvent.click(screen.getByRole("button", { name: /Send/i }));

    await waitFor(() => expect(textarea).toHaveValue(""));
  });
});
