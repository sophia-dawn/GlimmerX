import { QueryClient } from "@tanstack/react-query";

/**
 * The single query cache for the whole app.
 *
 * Lives in its own module (instead of `App.tsx`) so the lock path can clear it
 * without importing a React component: after the database key is dropped, any
 * balances still sitting in this cache are the only copy left in memory.
 *
 * Defaults mirror `QUERY_CONFIG.FINANCIAL`: never serve stale financial data,
 * but do not refetch the whole screen every time the window regains focus
 * (single-writer desktop app; mutations invalidate explicitly).
 */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 0,
      gcTime: 5 * 60 * 1000,
      refetchOnMount: "always",
      refetchOnWindowFocus: false,
    },
  },
});
