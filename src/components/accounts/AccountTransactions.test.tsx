import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { AccountTransactions } from "@/components/accounts/AccountTransactions";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, vars?: Record<string, unknown>) => {
      if (key === "accounts.transactions.forAccount" && vars?.name)
        return `for: ${vars.name}`;
      return key;
    },
  }),
}));

vi.mock("@/utils/api", () => ({
  accountTransactions: vi.fn().mockResolvedValue([]),
}));

vi.mock("@/utils/format", () => ({
  formatAmount: (n: number) => (n >= 0 ? `+${n}` : `${n}`),
}));

import { accountTransactions } from "@/utils/api";
const mockAccountTransactions = vi.mocked(accountTransactions);

function renderWithQueryClient(ui: React.ReactNode) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  });
  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
}

describe("AccountTransactions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders dialog with header", () => {
    renderWithQueryClient(
      <AccountTransactions
        accountId="1"
        accountName="Test Account"
        open={true}
        onOpenChange={vi.fn()}
      />,
    );
    expect(screen.getByText("accounts.transactions.title")).toBeInTheDocument();
    expect(screen.getByText("for: Test Account")).toBeInTheDocument();
  });

  it("renders date filter inputs and buttons", () => {
    renderWithQueryClient(
      <AccountTransactions
        accountId="1"
        accountName="Test"
        open={true}
        onOpenChange={vi.fn()}
      />,
    );
    expect(
      screen.getByText("accounts.transactions.fromDate"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("accounts.transactions.toDate"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("accounts.transactions.filter"),
    ).toBeInTheDocument();
    expect(screen.getByText("accounts.transactions.clear")).toBeInTheDocument();
  });

  it("shows loading state", () => {
    mockAccountTransactions.mockReturnValue(new Promise(() => {}));
    renderWithQueryClient(
      <AccountTransactions
        accountId="1"
        accountName="Test"
        open={true}
        onOpenChange={vi.fn()}
      />,
    );
    expect(screen.getByText("common.loading")).toBeInTheDocument();
  });

  it("shows empty state when no transactions", async () => {
    mockAccountTransactions.mockResolvedValue([]);
    renderWithQueryClient(
      <AccountTransactions
        accountId="1"
        accountName="Test"
        open={true}
        onOpenChange={vi.fn()}
      />,
    );
    await waitFor(() => {
      expect(
        screen.getByText("accounts.transactions.noTransactions"),
      ).toBeInTheDocument();
    });
  });

  it("renders transaction rows when data exists", async () => {
    mockAccountTransactions.mockResolvedValue([
      {
        id: "1",
        date: "2026-04-01",
        description: "Income",
        amount: 5000,
        category_id: null,
        created_at: "2026-04-01T00:00:00Z",
        updated_at: "2026-04-01T00:00:00Z",
      },
      {
        id: "2",
        date: "2026-04-02",
        description: "Expense",
        amount: -2000,
        category_id: null,
        created_at: "2026-04-02T00:00:00Z",
        updated_at: "2026-04-02T00:00:00Z",
      },
    ]);
    renderWithQueryClient(
      <AccountTransactions
        accountId="1"
        accountName="Test"
        open={true}
        onOpenChange={vi.fn()}
      />,
    );
    await waitFor(() => {
      expect(screen.getByText("Income")).toBeInTheDocument();
      expect(screen.getByText("Expense")).toBeInTheDocument();
      expect(screen.getByText("+5000")).toBeInTheDocument();
      expect(screen.getByText("-2000")).toBeInTheDocument();
    });
  });

  it("clears date filters when clear button clicked", () => {
    renderWithQueryClient(
      <AccountTransactions
        accountId="1"
        accountName="Test"
        open={true}
        onOpenChange={vi.fn()}
      />,
    );
    const fromDateInput = screen.getAllByRole("textbox")[0]!;
    fireEvent.change(fromDateInput, { target: { value: "2026-01-01" } });
    expect(fromDateInput).toHaveValue("2026-01-01");

    fireEvent.click(screen.getByText("accounts.transactions.clear"));
    expect(fromDateInput).toHaveValue("");
  });
});
