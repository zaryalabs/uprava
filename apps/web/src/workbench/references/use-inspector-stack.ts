import { useCallback } from "react";
import { useSearchParams } from "react-router-dom";

import type { CortexRef } from "../../shared/protocol/types";
import { pushInspectorRef } from "./refs";

export function useOpenReference() {
  const [, setSearchParams] = useSearchParams();
  return useCallback(
    (ref: CortexRef) => {
      setSearchParams((current) => pushInspectorRef(current, ref));
    },
    [setSearchParams],
  );
}
