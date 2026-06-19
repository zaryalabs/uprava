import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ShieldCheck } from "lucide-react";
import { useState } from "react";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type {
  NodeEnrollmentRequestedResponse,
  NodeEnrollmentSummary,
} from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import {
  canRunCommand,
  runWorkbenchCommand,
} from "../../workbench/commands/registry";

export function NodeEnrollmentPanel() {
  const queryClient = useQueryClient();
  const [displayName, setDisplayName] = useState("Local Node");
  const [createdEnrollment, setCreatedEnrollment] =
    useState<NodeEnrollmentRequestedResponse | null>(null);
  const enrollments = useQuery({
    queryKey: queryKeys.nodeEnrollments,
    queryFn: coreApi.nodeEnrollments,
  });
  const invalidateEnrollments = async () => {
    await queryClient.invalidateQueries({
      queryKey: queryKeys.nodeEnrollments,
    });
    await queryClient.invalidateQueries({ queryKey: queryKeys.inventory });
  };
  const createEnrollment = useMutation({
    mutationFn: () =>
      runWorkbenchCommand("node.createEnrollment", {
        nodeDisplayName: displayName,
        afterSuccess: invalidateEnrollments,
      }),
    onSuccess: (response) => {
      if (isCreatedEnrollment(response)) {
        setCreatedEnrollment(response);
      }
    },
  });
  const approveEnrollment = useMutation({
    mutationFn: (enrollmentId: string) =>
      runWorkbenchCommand("node.approveEnrollment", {
        enrollmentId,
        afterSuccess: invalidateEnrollments,
      }),
  });

  return (
    <section className="space-y-3 rounded-md border border-[#d9ded4] bg-white p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold uppercase tracking-normal text-[#667268]">
            Pair Node
          </h2>
          <p className="mt-1 text-sm text-[#536257]">
            Trusted local or controlled development pairing only.
          </p>
        </div>
        <Badge tone="warn">not production-secure</Badge>
      </div>

      <form
        className="grid gap-2 sm:grid-cols-[minmax(0,1fr)_auto]"
        onSubmit={(event) => {
          event.preventDefault();
          if (
            !canRunCommand("node.createEnrollment", {
              nodeDisplayName: displayName,
            })
          ) {
            return;
          }
          createEnrollment.mutate();
        }}
      >
        <label className="block space-y-1">
          <span className="text-sm font-medium">Display name</span>
          <input
            className="h-10 w-full rounded-md border border-[#bfc8bc] px-3"
            value={displayName}
            onChange={(event) => setDisplayName(event.target.value)}
          />
        </label>
        <div className="flex items-end">
          <Button
            variant="primary"
            disabled={
              createEnrollment.isPending ||
              !canRunCommand("node.createEnrollment", {
                nodeDisplayName: displayName,
              })
            }
          >
            <ShieldCheck size={15} />
            Create
          </Button>
        </div>
      </form>

      {createdEnrollment ? (
        <div className="rounded-md border border-[#a9c3d8] bg-[#e8f2fa] p-3 text-sm text-[#315d7d]">
          <div className="font-medium">Pairing code</div>
          <div className="mt-1 font-mono text-base">
            {createdEnrollment.pairing_code}
          </div>
          <div className="mt-1 text-xs">
            Expires {new Date(createdEnrollment.expires_at).toLocaleString()}
          </div>
        </div>
      ) : null}

      {createEnrollment.isError ? (
        <ErrorNotice
          error={createEnrollment.error}
          title="Enrollment request failed"
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
  const approved = isEnrollmentApproved(enrollment);

  return (
    <article className="rounded-md border border-[#e0e5dc] bg-[#f8faf5] p-3">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="truncate text-sm font-medium">
            {enrollment.display_name}
          </div>
          <div className="mt-1 font-mono text-xs text-[#667268]">
            {enrollment.enrollment_id}
          </div>
        </div>
        <Badge tone={enrollmentTone(enrollment)}>{enrollment.status}</Badge>
      </div>
      <div className="mt-2 flex flex-wrap items-center justify-between gap-2 text-xs text-[#536257]">
        <span>
          {approved
            ? "Approved; waiting for claim"
            : `Expires ${new Date(enrollment.expires_at).toLocaleString()}`}
        </span>
        <Button
          variant="secondary"
          disabled={!approvable || approving}
          onClick={onApprove}
        >
          Approve
        </Button>
      </div>
    </article>
  );
}

export function isEnrollmentApprovable(enrollment: NodeEnrollmentSummary) {
  return (
    enrollment.status === "pending_user_approval" && !enrollment.approved_at
  );
}

export function isEnrollmentApproved(enrollment: NodeEnrollmentSummary) {
  return enrollment.status === "approved" || Boolean(enrollment.approved_at);
}

function enrollmentTone(enrollment: NodeEnrollmentSummary) {
  if (enrollment.status === "registered") return "good";
  if (enrollment.status === "expired" || enrollment.status === "revoked") {
    return "bad";
  }
  return isEnrollmentApproved(enrollment) ? "info" : "warn";
}

function isCreatedEnrollment(
  value: unknown,
): value is NodeEnrollmentRequestedResponse {
  return (
    typeof value === "object" &&
    value !== null &&
    "pairing_code" in value &&
    typeof value.pairing_code === "string"
  );
}
