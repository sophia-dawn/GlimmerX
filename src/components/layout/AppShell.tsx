import { useCallback, useEffect, useRef, useState } from "react";
import { Outlet, useLocation } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import { Sidebar } from "./Sidebar";
import { Header } from "./Header";
import { QuickAddDialog } from "@/components/transactions/QuickAddDialog";
import { HeaderProvider } from "@/contexts/HeaderContext";

export function AppShell() {
  const location = useLocation();
  const queryClient = useQueryClient();
  const prevPathRef = useRef(location.pathname);
  const [quickAddOpen, setQuickAddOpen] = useState(false);

  useEffect(() => {
    if (prevPathRef.current !== location.pathname) {
      prevPathRef.current = location.pathname;
      queryClient.invalidateQueries();
    }
  }, [location.pathname, queryClient]);

  const openQuickAdd = useCallback(() => setQuickAddOpen(true), []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "N") {
        e.preventDefault();
        openQuickAdd();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [openQuickAdd]);

  return (
    <HeaderProvider>
      <div className="flex h-screen w-full overflow-hidden">
        <Sidebar />
        <div className="flex flex-1 flex-col overflow-hidden">
          <Header onQuickAdd={() => setQuickAddOpen(true)} />
          <main className="flex-1 overflow-auto p-2">
            <Outlet />
          </main>
        </div>
        <QuickAddDialog open={quickAddOpen} onOpenChange={setQuickAddOpen} />
      </div>
    </HeaderProvider>
  );
}
