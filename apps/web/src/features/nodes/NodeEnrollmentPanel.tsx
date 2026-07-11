import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ShieldCheck } from "lucide-react";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type { NodeEnrollmentSummary } from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { runWorkbenchCommand } from "../../workbench/commands/registry";

export function NodeEnrollmentPanel() {
  const queryClient = useQueryClient();
  const enrollments = useQuery({
    queryKey: queryKeys.nodeEnrollments,
    queryFn: coreApi.nodeEnrollments,
    refetchInterval: (query) =>
      query.state.status === "error" ? 15_000 : 5_000,
  });
  const invalidateEnrollments = async () => {
    await queryClient.invalidateQueries({
      queryKey: queryKeys.nodeEnrollments,
    });
    await queryClient.invalidateQueries({ queryKey: queryKeys.inventory });
  };
  const approveEnrollment = useMutation({
    mutationFn: (enrollmentId: string) =>
      runWorkbenchCommand("node.approveEnrollment", {
        enrollmentId,
        afterSuccess: invalidateEnrollments,
      }),
  });

  return (
    <section className="space-y-3 border border-[var(--color-muted)] bg-[var(--color-bg)] p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold uppercase tracking-normal text-[var(--color-muted)]">
            Pair Node
          </h2>
          <p className="mt-1 text-sm text-[var(--color-muted)]">
            Trusted local or controlled development pairing only.
          </p>
        </div>
        <Badge tone="warn">not production-secure</Badge>
      </div>

      {enrollments.isError ? (
        <ErrorNotice
          error={enrollments.error}
          title="Enrollment list unavailable"
        />
      ) : null}
      {approveEnrollment.isError ? (
        <ErrorNotice
          error={approveEnrollment.error}
          title="Enrollment approval failed"
        />
      ) : null}

      <div className="grid gap-2">
        {enrollments.data?.slice(0, 6).map((enrollment) => (
          <EnrollmentRow
            key={enrollment.enrollment_id}
            enrollment={enrollment}
            approving={approveEnrollment.isPending}
            onApprove={() => approveEnrollment.mutate(enrollment.enrollment_id)}
          />
        ))}
      </div>
      {enrollments.isLoading ? (
        <div className="border border-[var(--color-muted)] bg-[var(--color-bg-muted)] p-3 text-sm text-[var(--color-muted)]">
          Loading enrollment requests
        </div>
      ) : null}
      {!enrollments.isLoading && enrollments.data?.length === 0 ? (
        <div className="border border-[var(--color-muted)] bg-[var(--color-bg-muted)] p-3 text-sm text-[var(--color-muted)]">
          Waiting for local Node enrollment requests.
        </div>
      ) : null}
    </section>
  );
}

function EnrollmentRow({
  enrollment,
  approving,
  onApprove,
}: {
  enrollment: NodeEnrollmentSummary;
  approving: boolean;
  onApprove: () => void;
}) {
  const approvable = isEnrollmentApprovable(enrollment);

  return (
    <article className="border border-[var(--color-muted)] bg-[var(--color-bg-muted)] p-3">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="truncate text-sm font-medium">
            {enrollment.display_name}
          </div>
          <div className="mt-1 font-mono text-xs text-[var(--color-muted)]">
            {enrollment.enrollment_id}
          </div>
        </div>
        <Badge tone={enrollmentTone(enrollment)}>{enrollment.status}</Badge>
      </div>
      <div className="mt-2 flex flex-wrap items-center justify-between gap-2 text-xs text-[var(--color-muted)]">
        <span>{enrollmentStatusText(enrollment)}</span>
        <Button
          variant="secondary"
          disabled={!approvable || approving}
          onClick={onApprove}
        >
          <ShieldCheck size={15} />
          Approve
        </Button>
      </div>
    </article>
  );
}

export function isEnrollmentApprovable(enrollment: NodeEnrollmentSummary) {
  return (
    enrollment.status === "pending_user_approval" &&
    !enrollment.approved_at &&
    !isEnrollmentTerminal(enrollment)
  );
}

export function isEnrollmentApproved(enrollment: NodeEnrollmentSummary) {
  return (
    !isEnrollmentTerminal(enrollment) &&
    (enrollment.status === "approved" || Boolean(enrollment.approved_at))
  );
}

export function enrollmentStatusText(enrollment: NodeEnrollmentSummary) {
  if (enrollment.status === "registered") {
    return enrollment.claimed_node_id
      ? `Claimed by ${enrollment.claimed_node_id}`
      : "Claimed";
  }
  if (enrollment.status === "expired") return "Expired before claim";
  if (enrollment.status === "revoked") return "Revoked";
  if (enrollment.status === "rejected") return "Rejected";
  if (isEnrollmentApproved(enrollment)) {
    return "Approved; waiting for claim";
  }
  return `Expires ${new Date(enrollment.expires_at).toLocaleString()}`;
}

function enrollmentTone(enrollment: NodeEnrollmentSummary) {
  if (enrollment.status === "registered") return "good";
  if (isEnrollmentTerminal(enrollment)) {
    return "bad";
  }
  return isEnrollmentApproved(enrollment) ? "info" : "warn";
}

function isEnrollmentTerminal(enrollment: NodeEnrollmentSummary) {
  return (
    enrollment.status === "expired" ||
    enrollment.status === "revoked" ||
    enrollment.status === "rejected"
  );
}
