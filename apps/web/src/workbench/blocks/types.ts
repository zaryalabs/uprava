import type { UpravaRef } from "../../shared/protocol/types";

export type UiBlock = {
  block_id: string;
  type: string;
  schema_version: number;
  surface_id: string;
  primary_ref: UpravaRef;
  parent_ref?: UpravaRef | null;
  children: UiBlock[];
  source_refs: UpravaRef[];
  evidence_refs: UpravaRef[];
  cause_refs: UpravaRef[];
  related_refs: UpravaRef[];
  trace_refs: UpravaRef[];
  data: unknown;
  actions: string[];
  fallback_text?: string | null;
};
