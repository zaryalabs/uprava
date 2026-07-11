import { apiBase } from "../api/config";
import { readCookie } from "../auth/cookies";
import type { ClientLogLevel, ClientLogRequest } from "../protocol/types";

const MAX_MESSAGE_CHARS = 2_000;
const MAX_DETAIL_CHARS = 8_000;

let installed = false;
const sessionId = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}`;

export function installClientLogging() {
  if (installed || typeof window === "undefined") {
    return;
  }
  installed = true;
  logClientEvent("info", "web.bootstrap", "client logging initialized", {
    session_id: sessionId,
  });
  window.addEventListener("error", (event) => {
    logClientEvent("error", "web.global_error", event.message, {
      filename: event.filename,
      lineno: event.lineno,
      colno: event.colno,
      error: errorToLogDetail(event.error),
    });
  });
  window.addEventListener("unhandledrejection", (event) => {
    logClientEvent("error", "web.unhandled_rejection", "Unhandled rejection", {
      reason: errorToLogDetail(event.reason),
    });
  });
}

export function logClientEvent(
  level: ClientLogLevel,
  source: string,
  message: string,
  detail: unknown = {},
) {
  if (typeof window === "undefined") {
    return;
  }
  const csrf = readCookie("uprava_csrf");
  if (!csrf) {
    return;
  }
  const request: ClientLogRequest = {
    level,
    source: truncate(source, MAX_MESSAGE_CHARS),
    message: truncate(message, MAX_MESSAGE_CHARS),
    route: `${window.location.pathname}${window.location.search}`,
    user_agent: window.navigator.userAgent,
    occurred_at: new Date().toISOString(),
    detail: boundedDetail({
      session_id: sessionId,
      detail,
    }),
  };
  const body = JSON.stringify(request);
  const url = `${apiBase}/client/logs`;
  void fetch(url, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "x-uprava-csrf": csrf,
    },
    body,
    credentials: "include",
    keepalive: true,
  }).catch(() => undefined);
}

function boundedDetail(value: unknown) {
  const serialized = safeStringify(value);
  if (serialized.length <= MAX_DETAIL_CHARS) {
    return value;
  }
  return {
    truncated: true,
    text: serialized.slice(0, MAX_DETAIL_CHARS),
  };
}

function errorToLogDetail(error: unknown) {
  if (error instanceof Error) {
    return {
      name: error.name,
      message: error.message,
      stack: error.stack,
    };
  }
  return error;
}

function safeStringify(value: unknown) {
  const seen = new WeakSet<object>();
  try {
    return (
      JSON.stringify(value, (_key, nested) => {
        if (typeof nested === "object" && nested !== null) {
          if (seen.has(nested)) {
            return "[Circular]";
          }
          seen.add(nested);
        }
        if (typeof nested === "function") {
          return `[Function ${nested.name || "anonymous"}]`;
        }
        return nested;
      }) ?? "undefined"
    );
  } catch (error) {
    return String(error);
  }
}

function truncate(value: string, maxChars: number) {
  if (value.length <= maxChars) {
    return value;
  }
  return `${value.slice(0, maxChars)}...`;
}
