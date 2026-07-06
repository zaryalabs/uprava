import { useCallback } from "react";
import { useSearchParams } from "react-router-dom";

import type { UpravaRef } from "../../shared/protocol/types";
import { pushInspectorRef } from "./refs";

export function useOpenReference() {
  const [, setSearchParams] = useSearchParams();
  return useCallback(
    (ref: UpravaRef) => {
      setSearchParams((current) => pushInspectorRef(current, ref));
    },
    [setSearchParams],
  );
}
