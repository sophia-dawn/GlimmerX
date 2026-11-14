import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { QueryClientProvider } from "@tanstack/react-query";
import { AppShell } from "@/components/layout/AppShell";
import { UnlockPage } from "@/pages/UnlockPage";
import { OnboardingPage } from "@/pages/OnboardingPage";
import { DashboardPage } from "@/pages/DashboardPage";
import { AccountsPage } from "@/pages/AccountsPage";
import { TransactionsPage } from "@/pages/TransactionsPage";
import { TransactionDetailPage } from "@/pages/TransactionDetailPage";
import { CategoriesPage } from "@/pages/CategoriesPage";
import { BudgetsPage } from "@/pages/BudgetsPage";
import { SavingsPage } from "@/pages/SavingsPage";
import { SettingsPage } from "@/pages/SettingsPage";
import { ReportsPage } from "@/pages/ReportsPage";
import { ToasterProvider } from "@/components/ui/ToasterProvider";
import { useDbStore } from "@/stores/dbStore";
import { useAutoLock } from "@/hooks/useAutoLock";
import { queryClient } from "@/lib/queryClient";

function AppRoutes() {
  const { isUnlocked } = useDbStore();
  // Idle lock. Lives above the routes so it keeps running while the user
  // navigates, and unmounts nothing when it fires (the guard only navigates).
  useAutoLock();

  return (
    <Routes>
      <Route path="/unlock" element={<UnlockPage />} />
      <Route
        path="/onboarding"
        element={isUnlocked ? <OnboardingPage /> : <Navigate to="/unlock" />}
      />
      <Route
        path="/"
        element={isUnlocked ? <AppShell /> : <Navigate to="/unlock" />}
      >
        <Route index element={<DashboardPage />} />
        <Route path="accounts" element={<AccountsPage />} />
        <Route path="transactions" element={<TransactionsPage />} />
        <Route path="transactions/:id" element={<TransactionDetailPage />} />
        <Route path="categories" element={<CategoriesPage />} />
        <Route path="budgets" element={<BudgetsPage />} />
        <Route path="savings" element={<SavingsPage />} />
        <Route path="reports" element={<ReportsPage />} />
        <Route path="settings" element={<SettingsPage />} />
      </Route>
    </Routes>
  );
}

function App() {
  const { t } = useTranslation();
  const {
    isInitializing,
    setInitialized,
    checkExistingDb,
    restoreUnlockState,
  } = useDbStore();

  useEffect(() => {
    const init = async () => {
      const restored = await restoreUnlockState();
      if (!restored) {
        await checkExistingDb();
      }
      setInitialized();
    };
    init();
  }, [checkExistingDb, restoreUnlockState, setInitialized]);

  if (isInitializing) {
    return (
      <div className="flex h-screen w-full items-center justify-center bg-background">
        <div className="text-muted-foreground">{t("common.loading")}</div>
      </div>
    );
  }

  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <AppRoutes />
        <ToasterProvider />
      </BrowserRouter>
    </QueryClientProvider>
  );
}

export default App;
