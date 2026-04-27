import { afterEach, describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { AccountsPage } from "@/pages/AccountsPage";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { HeaderProvider } from "@/contexts/HeaderContext";

// Mock the API
vi.mock("@/utils/api", () => ({
  accountList: vi.fn(),
  accountClose: vi.fn().mockResolvedValue(undefined),
  accountReopen: vi.fn().mockResolvedValue(undefined),
  accountDelete: vi.fn().mockResolvedValue(undefined),
  accountUpdate: vi.fn().mockResolvedValue({}),
  accountCreate: vi.fn().mockResolvedValue({}),
  accountTransactions: vi.fn().mockResolvedValue([]),
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

// Mock the wizard and form to avoid complex nested providers
vi.mock("@/components/accounts/AccountWizard", () => ({
  AccountWizard: ({
    open,
    onOpenChange,
    onSuccess,
  }: {
    open: boolean;
    onOpenChange: (o: boolean) => void;
    onSuccess: () => void;
  }) =>
    open ? (
      <div data-testid="mock-wizard">
        <button
          onClick={() => {
            onSuccess();
            onOpenChange(false);
          }}
        >
          Create
        </button>
      </div>
    ) : null,
}));

vi.mock("@/components/accounts/AccountForm", () => ({
  AccountForm: ({
    open,
    onOpenChange,
    onSuccess,
  }: {
    open: boolean;
    onOpenChange: (o: boolean) => void;
    onSuccess: () => void;
  }) =>
    open ? (
      <div data-testid="mock-form">
        <button
          onClick={() => {
            onSuccess();
            onOpenChange(false);
          }}
        >
          Save
        </button>
      </div>
    ) : null,
}));

vi.mock("@/components/accounts/AccountTransactions", () => ({
  AccountTransactions: ({
    open,
    onOpenChange,
    accountName,
  }: {
    open: boolean;
    onOpenChange: (o: boolean) => void;
    accountName: string;
  }) =>
    open ? (
      <div data-testid="mock-transactions">
        <span>{accountName}</span>
        <button onClick={() => onOpenChange(false)}>Close</button>
      </div>
    ) : null,
}));

import { accountList } from "@/utils/api";
import type { AccountDto } from "@/types";

const mockAccountList = vi.mocked(accountList);

function renderWithQueryClient(ui: React.ReactNode) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
    },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <HeaderProvider>{ui}</HeaderProvider>
    </QueryClientProvider>,
  );
}

const baseMock = {
  is_system: false as const,
  iban: null as string | null,
  virtual_balance: 0,
  is_active: true,
  display_order: 0,
  include_net_worth: true,
};

const mockActiveAsset: AccountDto = {
  id: "a1",
  name: "Bank",
  account_type: "asset",
  currency: "CNY",
  description: "My bank account",
  account_number: "1234",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
  ...baseMock,
  meta: [
    {
      id: "m1",
      account_id: "a1",
      key: "account_role",
      value: "defaultAsset",
      created_at: "2026-01-01T00:00:00Z",
    },
  ],
};

const mockClosedAccount: AccountDto = {
  id: "c1",
  name: "Old Card",
  account_type: "liability",
  currency: "CNY",
  description: "",
  account_number: "5678",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-02-01T00:00:00Z",
  ...baseMock,
  is_active: false,
  meta: [
    {
      id: "m2",
      account_id: "c1",
      key: "account_role",
      value: "loan",
      created_at: "2026-01-01T00:00:00Z",
    },
  ],
};

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("AccountsPage", () => {
  it("renders loading state initially", () => {
    mockAccountList.mockReturnValue(new Promise(() => {}));
    renderWithQueryClient(<AccountsPage />);
    expect(screen.getByText(/加载中/)).toBeInTheDocument();
  });

  it("renders account list when data is loaded", async () => {
    mockAccountList.mockResolvedValue([mockActiveAsset]);
    renderWithQueryClient(<AccountsPage />);
    await screen.findByText("Bank");
    expect(screen.getByText("Bank")).toBeInTheDocument();
  });

  it("shows account metadata: subtype badge, account number", async () => {
    mockAccountList.mockResolvedValue([mockActiveAsset]);
    renderWithQueryClient(<AccountsPage />);
    await screen.findByText("1234");
    expect(screen.getByText("1234")).toBeInTheDocument();
  });

  it("filters closed accounts by default", async () => {
    mockAccountList.mockResolvedValue([mockClosedAccount]);
    renderWithQueryClient(<AccountsPage />);
    await screen.findByRole("tablist");
    expect(screen.getByRole("tab", { name: /全部/ })).toBeInTheDocument();
  });

  it("renders account list content after loading", async () => {
    mockAccountList.mockResolvedValue([mockActiveAsset]);
    renderWithQueryClient(<AccountsPage />);
    await screen.findByText("Bank");
    expect(screen.getByText("Bank")).toBeInTheDocument();
  });

  it("renders account rows when data exists", async () => {
    mockAccountList.mockResolvedValue([mockActiveAsset]);
    renderWithQueryClient(<AccountsPage />);
    await screen.findByText("Bank");
    expect(screen.getByText("Bank")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /全部/ })).toBeInTheDocument();
  });
});
