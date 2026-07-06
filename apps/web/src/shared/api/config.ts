export const apiBase =
  import.meta.env.VITE_UPRAVA_API_BASE?.toString() ??
  "http://127.0.0.1:8080/api/v1";
