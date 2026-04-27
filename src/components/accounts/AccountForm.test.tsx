import { afterEach, describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { AccountForm } from "@/components/accounts/AccountForm";
import type { AccountDto } from "@/types";

vi.mock("@/utils/api", () => ({
  accountUpdate: vi.fn().mockResolvedValue({}),
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

const mockAccountDto: AccountDto = {
  id: "test-1",
  name: "Test Account",
  account_type: "asset" as const,
  currency: "CNY",
  description: "A test account",
  account_number: "1234567890",
  is_system: false,
  iban: null,
  is_active: true,
  include_net_worth: true,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
  meta: [
    {
      id: "m1",
      account_id: "test-1",
      key: "account_role",
      value: "defaultAsset",
      created_at: "2026-01-01T00:00:00Z",
    },
  ],
};

const defaultProps = {
  open: true,
  onOpenChange: vi.fn(),
  onSuccess: vi.fn(),
  editNode: null as AccountDto | null,
};

afterEach(() => {
  vi.clearAllMocks();
});

describe("AccountForm", () => {
  it("returns null when not editing", () => {
    const { container } = render(<AccountForm {...defaultProps} />);
    expect(container.innerHTML).toBe("");
  });

  it("renders all fields when editing an asset account", () => {
    render(<AccountForm {...defaultProps} editNode={mockAccountDto} />);
    expect(screen.getByLabelText(/账户名称/)).toBeInTheDocument();
    expect(screen.getByLabelText(/备注/)).toBeInTheDocument();
    expect(screen.getByLabelText(/账户号/)).toBeInTheDocument();
    expect(screen.getByText(/子类型/)).toBeInTheDocument();
    expect(screen.getByText(/CNY/)).toBeInTheDocument();
  });

  it("pre-fills subtype value correctly", () => {
    render(<AccountForm {...defaultProps} editNode={mockAccountDto} />);
    // Radix Select renders the value in the trigger and as an <option>
    const subtypeElements = screen.getAllByText(/默认资产/);
    expect(subtypeElements.length).toBeGreaterThanOrEqual(1);
  });

  it("pre-fills form values from editNode", () => {
    render(<AccountForm {...defaultProps} editNode={mockAccountDto} />);
    expect((screen.getByLabelText(/账户名称/) as HTMLInputElement).value).toBe(
      "Test Account",
    );
    expect((screen.getByLabelText(/账户号/) as HTMLInputElement).value).toBe(
      "1234567890",
    );
  });

  it("calls onSuccess and onOpenChange on submit", async () => {
    const onSuccess = vi.fn();
    const onOpenChange = vi.fn();
    render(
      <AccountForm
        {...defaultProps}
        editNode={mockAccountDto}
        onSuccess={onSuccess}
        onOpenChange={onOpenChange}
      />,
    );
    const saveBtn = screen.getByRole("button", { name: /保存/ });
    fireEvent.click(saveBtn);
    await waitFor(() => {
      expect(onSuccess).toHaveBeenCalled();
    });
  });

  it("does not show subtype selector for equity accounts", () => {
    const equityAccount: AccountDto = {
      ...mockAccountDto,
      account_type: "equity" as const,
      meta: [],
    };
    render(<AccountForm {...defaultProps} editNode={equityAccount} />);
    expect(() => screen.getByRole("combobox")).toThrow();
  });

  it("pre-fills liability_type for liability accounts", () => {
    const liabilityAccount: AccountDto = {
      ...mockAccountDto,
      account_type: "liability" as const,
      meta: [
        {
          id: "m1",
          account_id: "test-1",
          key: "liability_type",
          value: "debt",
          created_at: "2026-01-01T00:00:00Z",
        },
      ],
    };
    render(<AccountForm {...defaultProps} editNode={liabilityAccount} />);
    const subtypeElements = screen.getAllByText(/借款/);
    expect(subtypeElements.length).toBeGreaterThanOrEqual(1);
  });

  it("pre-fills legacy account_role for liability accounts (backwards compat)", () => {
    const liabilityAccount: AccountDto = {
      ...mockAccountDto,
      account_type: "liability" as const,
      meta: [
        {
          id: "m1",
          account_id: "test-1",
          key: "account_role",
          value: "loan",
          created_at: "2026-01-01T00:00:00Z",
        },
      ],
    };
    render(<AccountForm {...defaultProps} editNode={liabilityAccount} />);
    const subtypeElements = screen.getAllByText(/贷款/);
    expect(subtypeElements.length).toBeGreaterThanOrEqual(1);
  });
});
