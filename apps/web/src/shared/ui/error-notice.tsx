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
      className="rounded-md border border-[#dcaaa5] bg-[#fde5e2] p-3 text-sm text-[#88332f]"
    >
      <div className="font-medium">{title}</div>
      <div className="mt-1 break-words">
        {envelope?.message ?? "Request failed"}
      </div>
      {envelope ? (
        <div className="mt-2 space-y-1 font-mono text-xs text-[#6f3a37]">
          <div>{envelope.error_code}</div>
          <div>{envelope.correlation_id}</div>
        </div>
      ) : null}
    </div>
  );
}
