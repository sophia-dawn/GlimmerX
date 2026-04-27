import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import App from "@/App";

// Mock all child components and external deps
vi.mock("@/stores/dbStore", () => ({
  useDbStore: () => ({
    isUnlocked: true,
    checkExistingDb: vi.fn(),
  }),
}));

interface MockChildren {
  children?: ReactNode;
}

vi.mock("react-router-dom", () => {
  const actual = vi.importActual("react-router-dom");
  return {
    ...actual,
    BrowserRouter: ({ children }: MockChildren) => (
      <div data-testid="router">{children}</div>
    ),
    Routes: ({ children }: MockChildren) => (
      <div data-testid="routes">{children}</div>
    ),
    Route: () => null,
    Navigate: ({ to }: { to: string }) => (
      <div data-testid="navigate" data-to={to} />
    ),
    Outlet: () => <div data-testid="outlet" />,
  };
});

vi.mock("@tanstack/react-query", () => ({
  QueryClient: class {
    constructor() {}
  },
  QueryClientProvider: ({ children }: MockChildren) => (
    <div data-testid="query-provider">{children}</div>
  ),
}));

vi.mock("@/components/layout/AppShell", () => ({
  AppShell: () => <div data-testid="app-shell" />,
}));

vi.mock("@/pages/UnlockPage", () => ({
  UnlockPage: () => <div data-testid="unlock-page" />,
}));

vi.mock("@/pages/OnboardingPage", () => ({
  OnboardingPage: () => <div data-testid="onboarding-page" />,
}));

vi.mock("@/pages/DashboardPage", () => ({
  DashboardPage: () => <div data-testid="dashboard-page" />,
}));

vi.mock("@/pages/AccountsPage", () => ({
  AccountsPage: () => <div data-testid="accounts-page" />,
}));

vi.mock("@/components/layout/PlaceholderPage", () => ({
  PlaceholderPage: ({ title }: { title: string }) => (
    <div data-testid="placeholder-page">{title}</div>
  ),
}));

vi.mock("@/pages/SettingsPage", () => ({
  SettingsPage: () => <div data-testid="settings-page" />,
}));

vi.mock("@/components/ui/ToasterProvider", () => ({
  ToasterProvider: () => <div data-testid="toaster" />,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

// Re-import mocked module
import { useDbStore } from "@/stores/dbStore";

describe("App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders with QueryClientProvider and BrowserRouter", () => {
    render(<App />);
    expect(screen.getByTestId("query-provider")).toBeInTheDocument();
    expect(screen.getByTestId("router")).toBeInTheDocument();
  });

  it("renders ToasterProvider", () => {
    render(<App />);
    expect(screen.getByTestId("toaster")).toBeInTheDocument();
  });

  it("calls checkExistingDb on mount", () => {
    const checkExistingDb = vi.fn();
    vi.mocked(useDbStore).mockReturnValue({
      isUnlocked: true,
      checkExistingDb,
    });
    render(<App />);
    expect(checkExistingDb).toHaveBeenCalled();
  });
});
