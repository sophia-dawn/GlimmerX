import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AppShell } from "@/components/layout/AppShell";
import { UnlockPage } from "@/pages/UnlockPage";
import { OnboardingPage } from "@/pages/OnboardingPage";
import { DashboardPage } from "@/pages/DashboardPage";
import { AccountsPage } from "@/pages/AccountsPage";
import { TransactionsPage } from "@/pages/TransactionsPage";
import { TransactionDetailPage } from "@/pages/TransactionDetailPage";
import { CategoriesPage } from "@/pages/CategoriesPage";
import { BudgetsPage } from "@/pages/BudgetsPage";
import { SettingsPage } from "@/pages/SettingsPage";
import { ReportsPage } from "@/pages/ReportsPage";
import { ToasterProvider } from "@/components/ui/ToasterProvider";
import { useDbStore } from "@/stores/dbStore";

const queryClient = new QueryClient();

function AppRoutes() {
  const { isUnlocked } = useDbStore();

  console.log("[App] AppRoutes render, isUnlocked:", isUnlocked);

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
    isUnlocked,
    setInitialized,
    checkExistingDb,
    restoreUnlockState,
  } = useDbStore();

  console.log(
    "[App] render, isInitializing:",
    isInitializing,
    "isUnlocked:",
    isUnlocked,
  );

  useEffect(() => {
    const init = async () => {
      console.log("[App] init start");
      const restored = await restoreUnlockState();
      console.log("[App] restoreUnlockState result:", restored);
      if (!restored) {
        await checkExistingDb();
      }
      setInitialized();
      console.log("[App] init complete");
    };
    init();
  }, [checkExistingDb, restoreUnlockState, setInitialized]);

  if (isInitializing) {
    console.log("[App] showing loading screen");
    return (
      <div className="flex h-screen w-full items-center justify-center bg-background">
        <div className="text-muted-foreground">{t("common.loading")}</div>
      </div>
    );
  }

  console.log("[App] rendering main app");
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
