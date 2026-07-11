import { UpravaApiError } from "../api/http-client";

type ErrorNoticeProps = {
  error: unknown;
  title: string;
};

export function ErrorNotice({ error, title }: ErrorNoticeProps) {
  const envelope = error instanceof UpravaApiError ? error.envelope : null;

  return (
    <div
      role="alert"
      className="border-l-2 border-[var(--color-risk)] bg-[var(--color-risk-soft)] p-3 text-sm text-[var(--color-risk)]"
    >
      <div className="font-bold">{title}</div>
      <div className="mt-1 break-words">
        {envelope?.message ?? "Request failed. Check the connection and retry."}
      </div>
      {envelope ? (
        <div className="mt-2 space-y-1 font-mono text-xs">
          <div>{envelope.error_code}</div>
          <div>{envelope.correlation_id}</div>
        </div>
      ) : null}
    </div>
  );
}
