import { useCallback, useEffect, useState } from "react";
import { Outlet } from "react-router-dom";
import { Sidebar } from "./Sidebar";
import { Header } from "./Header";
import { QuickAddDialog } from "@/components/transactions/QuickAddDialog";
import { AiInputDialog } from "@/components/transactions/AiInputDialog";
import { HeaderProvider } from "@/contexts/HeaderContext";
import { useUiStore } from "@/stores/uiStore";

export function AppShell() {
  const quickAddOpen = useUiStore((state) => state.quickAddOpen);
  const setQuickAddOpen = useUiStore((state) => state.setQuickAddOpen);
  const [aiInputOpen, setAiInputOpen] = useState(false);

  const openAiInput = useCallback(() => setAiInputOpen(true), []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "I") {
        e.preventDefault();
        openAiInput();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [openAiInput]);

  return (
    <HeaderProvider>
      {/* `h-screen … overflow-hidden` is the window frame. Print cannot use a
          frame — the page has to grow — so the three elements below carry
          `data-print` for the rules in globals.css that unclip them. */}
      <div data-print="shell" className="flex h-screen w-full overflow-hidden">
        <Sidebar />
        <div data-print="pane" className="flex flex-1 flex-col overflow-hidden">
          <Header
            onQuickAdd={() => setQuickAddOpen(true)}
            onAiInput={() => setAiInputOpen(true)}
          />
          <main data-print="page" className="flex-1 overflow-auto p-3">
            <Outlet />
          </main>
        </div>
        <QuickAddDialog open={quickAddOpen} onOpenChange={setQuickAddOpen} />
        <AiInputDialog open={aiInputOpen} onOpenChange={setAiInputOpen} />
      </div>
    </HeaderProvider>
  );
}
