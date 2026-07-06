export const apiBase =
  import.meta.env.VITE_UPRAVA_API_BASE?.toString() ??
  "http://127.0.0.1:8080/api/v1";

export const apiWsBase = apiBase.replace(/^http/i, (protocol: string) =>
  protocol.toLowerCase() === "https" ? "wss" : "ws",
);
