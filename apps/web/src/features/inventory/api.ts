import { useQuery } from "@tanstack/react-query";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";

export function useInventory() {
  return useQuery({
    queryKey: queryKeys.inventory,
    queryFn: coreApi.inventory,
    refetchInterval: (query) =>
      query.state.status === "error" ? 15_000 : 5_000,
  });
}

export function useHealth() {
  return useQuery({
    queryKey: queryKeys.health,
    queryFn: coreApi.health,
  });
}

export function useVersion() {
  return useQuery({
    queryKey: queryKeys.version,
    queryFn: coreApi.version,
  });
}
