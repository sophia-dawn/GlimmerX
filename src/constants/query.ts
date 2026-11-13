/**
 * React Query configuration per AGENTS.md rule:
 * "the frontend must never render stale financial data"
 *
 * `staleTime: 0` guarantees every mount reads fresh values from the Rust
 * backend. `gcTime` only controls how long an *unmounted* query is retained and
 * cannot serve stale data, so a non-zero value avoids pointless refetch churn.
 */

export const QUERY_CONFIG = {
  // Financial data - never serve stale values
  FINANCIAL: {
    staleTime: 0,
    gcTime: 5 * 60 * 1000,
    refetchOnMount: "always" as const,
  },

  // Static metadata - caching acceptable
  STATIC: {
    staleTime: Infinity,
    gcTime: Infinity,
    refetchOnMount: false as const,
  },
} as const;
