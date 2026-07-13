import "@testing-library/jest-dom/vitest";

const storage = new Map<string, string>();
Object.defineProperty(window, "localStorage", {
  configurable: true,
  value: {
    clear: () => storage.clear(),
    getItem: (key: string) => storage.get(key) ?? null,
    key: (index: number) => [...storage.keys()][index] ?? null,
    get length() {
      return storage.size;
    },
    removeItem: (key: string) => storage.delete(key),
    setItem: (key: string, value: string) => storage.set(key, String(value)),
  } satisfies Storage,
});

function isMonacoCancellation(reason: unknown) {
  if (!reason || typeof reason !== "object") {
    return false;
  }
  const value = reason as { message?: unknown; name?: unknown };
  return value.message === "Canceled" || value.name === "Canceled";
}

window.addEventListener("unhandledrejection", (event) => {
  if (isMonacoCancellation(event.reason)) {
    event.preventDefault();
  }
});

process.on("unhandledRejection", (reason) => {
  if (isMonacoCancellation(reason)) {
    return;
  }
});

if (typeof document.queryCommandSupported !== "function") {
  Object.defineProperty(document, "queryCommandSupported", {
    configurable: true,
    value: () => false,
  });
}

if (typeof window.matchMedia !== "function") {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: (query: string): MediaQueryList =>
      ({
        matches: false,
        media: query,
        onchange: null,
        addEventListener: () => undefined,
        removeEventListener: () => undefined,
        addListener: () => undefined,
        removeListener: () => undefined,
        dispatchEvent: () => false,
      }) as MediaQueryList,
  });
}

Object.defineProperty(HTMLCanvasElement.prototype, "getContext", {
  configurable: true,
  value: () =>
    ({
      measureText: () => ({ width: 0 }),
    }) as unknown as CanvasRenderingContext2D,
});

if (typeof ResizeObserver === "undefined") {
  class TestResizeObserver implements ResizeObserver {
    constructor(private readonly callback: ResizeObserverCallback) {}

    disconnect() {}

    observe(target: Element) {
      this.callback(
        [
          {
            target,
            contentRect: {
              width: 800,
              height: 400,
              x: 0,
              y: 0,
              top: 0,
              right: 800,
              bottom: 400,
              left: 0,
              toJSON: () => ({}),
            },
          } as ResizeObserverEntry,
        ],
        this,
      );
    }

    unobserve() {}
  }

  Object.defineProperty(globalThis, "ResizeObserver", {
    configurable: true,
    value: TestResizeObserver,
  });
}
