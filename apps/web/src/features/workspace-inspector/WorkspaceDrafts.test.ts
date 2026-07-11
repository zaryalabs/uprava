import { act, renderHook, waitFor } from "@testing-library/react";
import { createElement, type ReactNode } from "react";
import { describe, expect, it } from "vitest";

import {
  receiveRemote,
  useWorkspaceDraft,
  WorkspaceDraftProvider,
  type WorkspaceDraft,
} from "./WorkspaceDrafts";

describe("receiveRemote", () => {
  it("updates a clean draft to the newest remote content", () => {
    expect(receiveRemote(cleanDraft("old"), "new")).toEqual(cleanDraft("new"));
  });

  it("preserves dirty local content and records a remote conflict", () => {
    expect(receiveRemote(dirtyDraft(), "remote changed")).toEqual({
      baseContent: "base",
      localContent: "local edit",
      dirty: true,
      conflict: true,
      remoteContent: "remote changed",
    });
  });

  it("does not fabricate a conflict when refetch returns the draft base", () => {
    expect(receiveRemote(dirtyDraft(), "base")).toEqual(dirtyDraft());
  });
});

describe("WorkspaceDraftProvider", () => {
  const wrapper = ({ children }: { children: ReactNode }) =>
    createElement(WorkspaceDraftProvider, null, children);

  it("preserves A drafts across A→B→A navigation", async () => {
    const { result, rerender } = renderHook(
      ({ path, remote }) => useWorkspaceDraft("placement-a", path, remote),
      {
        wrapper,
        initialProps: { path: "a.txt", remote: "base-a" },
      },
    );
    await waitFor(() =>
      expect(result.current.draft?.localContent).toBe("base-a"),
    );
    act(() => result.current.edit("local-a"));
    rerender({ path: "b.txt", remote: "base-b" });
    await waitFor(() =>
      expect(result.current.draft?.localContent).toBe("base-b"),
    );
    rerender({ path: "a.txt", remote: "base-a" });
    expect(result.current.draft).toMatchObject({
      localContent: "local-a",
      dirty: true,
    });
  });

  it("isolates the same path between Placements", async () => {
    const { result, rerender } = renderHook(
      ({ placement, remote }) =>
        useWorkspaceDraft(placement, "README.md", remote),
      {
        wrapper,
        initialProps: { placement: "placement-a", remote: "A" },
      },
    );
    await waitFor(() => expect(result.current.draft?.localContent).toBe("A"));
    act(() => result.current.edit("A edited"));
    rerender({ placement: "placement-b", remote: "B" });
    await waitFor(() => expect(result.current.draft?.localContent).toBe("B"));
    rerender({ placement: "placement-a", remote: "A" });
    expect(result.current.draft?.localContent).toBe("A edited");
  });

  it("supports conflict reload, discard, and unload protection", async () => {
    const { result, rerender } = renderHook(
      ({ remote }) => useWorkspaceDraft("placement-a", "a.txt", remote),
      { wrapper, initialProps: { remote: "base" } },
    );
    await waitFor(() =>
      expect(result.current.draft?.localContent).toBe("base"),
    );
    act(() => result.current.edit("local"));
    const unload = new Event("beforeunload", { cancelable: true });
    window.dispatchEvent(unload);
    expect(unload.defaultPrevented).toBe(true);

    rerender({ remote: "remote" });
    await waitFor(() => expect(result.current.draft?.conflict).toBe(true));
    act(() => result.current.reload());
    expect(result.current.draft).toMatchObject({
      baseContent: "remote",
      localContent: "remote",
      dirty: false,
      conflict: false,
    });
    act(() => result.current.edit("again"));
    act(() => result.current.discard());
    expect(result.current.draft?.localContent).toBe("remote");
    expect(result.current.draft?.dirty).toBe(false);
  });
});

function cleanDraft(content: string): WorkspaceDraft {
  return {
    baseContent: content,
    localContent: content,
    dirty: false,
    conflict: false,
    remoteContent: null,
  };
}

function dirtyDraft(): WorkspaceDraft {
  return {
    baseContent: "base",
    localContent: "local edit",
    dirty: true,
    conflict: false,
    remoteContent: null,
  };
}
