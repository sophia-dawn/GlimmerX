/**
 * React Query configuration per AGENTS.md rule:
 * "Frontend must not use any form of client-side caching"
 */

export const QUERY_CONFIG = {
  // Financial data - NO caching
  FINANCIAL: {
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always" as const,
    refetchOnWindowFocus: true,
  },

  // Static metadata - caching acceptable
  STATIC: {
    staleTime: Infinity,
    gcTime: Infinity,
    refetchOnMount: false,
  },
} as const;
