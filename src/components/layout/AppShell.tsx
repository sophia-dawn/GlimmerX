import { useCallback, useEffect, useState } from "react";
import { Outlet } from "react-router-dom";
import { Sidebar } from "./Sidebar";
import { Header } from "./Header";
import { QuickAddDialog } from "@/components/transactions/QuickAddDialog";
import { AiInputDialog } from "@/components/transactions/AiInputDialog";
import { HeaderProvider } from "@/contexts/HeaderContext";
import { useUiStore } from "@/stores/uiStore";
import { useSecurityStore } from "@/stores/securityStore";
import { matchesAccelerator } from "@/utils/shortcut";

export function AppShell() {
  // Quick-add open state lives in a store so the global shortcut (registered in
  // App, above the router) can summon it.
  const quickAddOpen = useUiStore((state) => state.quickAddOpen);
  const setQuickAddOpen = useUiStore((state) => state.setQuickAddOpen);
  const openQuickAdd = useUiStore((state) => state.openQuickAdd);
  const quickAddShortcut = useSecurityStore((state) => state.quickAddShortcut);
  const [aiInputOpen, setAiInputOpen] = useState(false);

  const openAiInput = useCallback(() => setAiInputOpen(true), []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Same accelerator as the system-wide registration: works while focused
      // even if the OS refused the global binding.
      if (matchesAccelerator(e, quickAddShortcut)) {
        e.preventDefault();
        openQuickAdd();
      }
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "I") {
        e.preventDefault();
        openAiInput();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [openQuickAdd, openAiInput, quickAddShortcut]);

  return (
    <HeaderProvider>
      <div className="flex h-screen w-full overflow-hidden">
        <Sidebar />
        <div className="flex flex-1 flex-col overflow-hidden">
          <Header
            onQuickAdd={() => setQuickAddOpen(true)}
            onAiInput={() => setAiInputOpen(true)}
          />
          <main className="flex-1 overflow-auto p-2">
            <Outlet />
          </main>
        </div>
        <QuickAddDialog open={quickAddOpen} onOpenChange={setQuickAddOpen} />
        <AiInputDialog open={aiInputOpen} onOpenChange={setAiInputOpen} />
      </div>
    </HeaderProvider>
  );
}
