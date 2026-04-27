import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { DashboardPage } from "@/pages/DashboardPage";

// Mock all dashboard hooks
vi.mock("@/hooks/useDashboardSummary", () => ({
  useDashboardSummary: () => ({
    data: {
      month_income: 100000,
      month_expense: 50000,
      month_start: "2026-04-01",
      month_end: "2026-04-30",
      year_income: 500000,
      year_expense: 200000,
      year_start: "2026-01-01",
      year_end: "2026-04-20",
      total_assets: 1000000,
      total_liabilities: 200000,
      net_worth: 800000,
      calculated_at: "2026-04-20T12:00:00Z",
    },
    isLoading: false,
    error: null,
  }),
}));

vi.mock("@/hooks/useMonthlyChart", () => ({
  useMonthlyChart: () => ({
    data: {
      year: 2026,
      month: 4,
      days: [],
      month_total_income: 0,
      month_total_expense: 0,
    },
    isLoading: false,
    error: null,
  }),
}));

vi.mock("@/hooks/useCategoryBreakdown", () => ({
  useCategoryBreakdown: () => ({
    data: {
      year: 2026,
      month: 4,
      category_type: "expense",
      categories: [],
      total_amount: 0,
    },
    isLoading: false,
    error: null,
  }),
}));

vi.mock("@/hooks/useTopExpenses", () => ({
  useTopExpenses: () => ({
    data: {
      year: 2026,
      month: 4,
      expenses: [],
    },
    isLoading: false,
    error: null,
  }),
}));

// Mock useQuery for components that use it directly
vi.mock("@tanstack/react-query", () => ({
  useQuery: vi.fn((options) => {
    const key = options.queryKey;
    if (key[0] === "recent-transactions") {
      return {
        data: {
          items: [],
          pagination: {
            page: 1,
            pageSize: 15,
            totalCount: 0,
            totalPages: 0,
            hasNext: false,
            hasPrev: false,
          },
          dateGroups: [],
        },
        isLoading: false,
        error: null,
      };
    }
    if (key[0] === "accounts") {
      return {
        data: [],
        isLoading: false,
        error: null,
      };
    }
    if (key[0] === "account-balance") {
      return {
        data: 0,
        isLoading: false,
        error: null,
      };
    }
    return { data: null, isLoading: false, error: null };
  }),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("react-router", () => ({
  Link: ({ children, to }: { children: React.ReactNode; to: string }) => (
    <a href={to}>{children}</a>
  ),
}));

vi.mock("recharts", () => ({
  BarChart: () => <div data-testid="bar-chart">BarChart</div>,
  Bar: () => <div>Bar</div>,
  XAxis: () => <div>XAxis</div>,
  YAxis: () => <div>YAxis</div>,
  CartesianGrid: () => <div>CartesianGrid</div>,
  Tooltip: () => <div>Tooltip</div>,
  Legend: () => <div>Legend</div>,
  ResponsiveContainer: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="responsive-container">{children}</div>
  ),
  PieChart: () => <div data-testid="pie-chart">PieChart</div>,
  Pie: () => <div>Pie</div>,
  Cell: () => <div>Cell</div>,
}));

describe("DashboardPage", () => {
  it("renders welcome heading", () => {
    render(<DashboardPage />);
    expect(screen.getByText("dashboard.welcome")).toBeInTheDocument();
  });

  it("renders subtitle", () => {
    render(<DashboardPage />);
    expect(screen.getByText("dashboard.subtitle")).toBeInTheDocument();
  });

  it("renders dashboard summary cards section", () => {
    render(<DashboardPage />);
    expect(screen.getByText("dashboard.monthIncome")).toBeInTheDocument();
    expect(screen.getByText("dashboard.monthExpense")).toBeInTheDocument();
    expect(screen.getByText("dashboard.yearIncome")).toBeInTheDocument();
    expect(screen.getByText("dashboard.yearExpense")).toBeInTheDocument();
  });

  it("renders financial health cards section", () => {
    render(<DashboardPage />);
    expect(screen.getByText("dashboard.totalAssets")).toBeInTheDocument();
    expect(screen.getByText("dashboard.totalDebt")).toBeInTheDocument();
    expect(screen.getByText("dashboard.netWorth")).toBeInTheDocument();
  });

  it("renders monthly chart section", () => {
    render(<DashboardPage />);
    expect(screen.getByText("dashboard.monthlyChart")).toBeInTheDocument();
  });

  it("renders category breakdown section", () => {
    render(<DashboardPage />);
    expect(screen.getByText("dashboard.categoryBreakdown")).toBeInTheDocument();
  });

  it("renders top expenses section", () => {
    render(<DashboardPage />);
    expect(screen.getByText("dashboard.topExpenses")).toBeInTheDocument();
  });

  it("renders recent transactions section", () => {
    render(<DashboardPage />);
    expect(
      screen.getByText("dashboard.recentTransactions"),
    ).toBeInTheDocument();
  });

  it("renders account balance list section", () => {
    render(<DashboardPage />);
    expect(
      screen.getByText("dashboard.accountBalanceList"),
    ).toBeInTheDocument();
  });
});
