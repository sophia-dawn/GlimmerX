import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AiInputDialog } from "./AiInputDialog";

// Mock the API module. Every function the rendered tree may touch must exist,
// because the confirmation card mounts QuickAddDialog (queries included).
vi.mock("@/utils/api", () => ({
  setUiLanguage: vi.fn().mockResolvedValue(undefined),
  aiParseTransaction: vi.fn(),
  quickAddTransaction: vi.fn(),
  accountCreate: vi.fn(),
  accountList: vi.fn().mockResolvedValue([]),
  categoryList: vi.fn().mockResolvedValue([]),
}));

// Mock error translation
vi.mock("@/utils/errorTranslation", () => ({
  translateErrorMessage: vi.fn((err) => String(err)),
}));

import { aiParseTransaction, accountCreate } from "@/utils/api";

const mockAiParse = aiParseTransaction as ReturnType<typeof vi.fn>;
const mockAccountCreate = accountCreate as ReturnType<typeof vi.fn>;

function renderWithProviders(ui: React.ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
}

const expenseProposal = {
  action: "expense",
  amount: "12.50",
  date: "2026-11-14",
  description: "lunch",
  sourceAccount: { id: "acc-1", name: "WeChat" },
  categoryName: "餐饮",
  warnings: ["分类「餐饮」是新建的"],
};

describe("AiInputDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders input and button when open", () => {
    renderWithProviders(<AiInputDialog open={true} onOpenChange={vi.fn()} />);
    expect(screen.getByPlaceholderText(/中午吃饭|Lunch/i)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /识别并录入|Parse/i }),
    ).toBeInTheDocument();
  });

  it("does not render when closed", () => {
    renderWithProviders(<AiInputDialog open={false} onOpenChange={vi.fn()} />);
    expect(
      screen.queryByPlaceholderText(/中午吃饭|Lunch/i),
    ).not.toBeInTheDocument();
  });

  it("submits text and moves to the confirmation card without closing", async () => {
    mockAiParse.mockResolvedValueOnce(expenseProposal);
    const onOpenChange = vi.fn();

    renderWithProviders(
      <AiInputDialog open={true} onOpenChange={onOpenChange} />,
    );

    const input = screen.getByPlaceholderText(/中午吃饭|Lunch/i);
    fireEvent.change(input, { target: { value: "中午吃饭18元" } });

    const button = screen.getByRole("button", { name: /识别并录入|Parse/i });
    fireEvent.click(button);

    await waitFor(() => {
      expect(mockAiParse).toHaveBeenCalledWith("中午吃饭18元");
    });

    // Recognition succeeded: the dialog must NOT close — the user still has
    // to confirm the proposal (design doc 25, §4.2).
    await waitFor(() => {
      expect(screen.getByText("确认记账")).toBeInTheDocument();
    });
    expect(onOpenChange).not.toHaveBeenCalledWith(false);

    // The proposal summary with its warning is visible on the card.
    expect(screen.getByTestId("proposal-review")).toBeInTheDocument();
    expect(screen.getByText(/分类「餐饮」是新建的/)).toBeInTheDocument();

    // The prefilled amount survived the dialog's reset effects.
    const amount = screen.getByLabelText(/金额|Amount/) as HTMLInputElement;
    expect(amount.value).toBe("12.50");
  });

  it("routes create_account proposals to the account confirmation card", async () => {
    mockAiParse.mockResolvedValueOnce({
      action: "create_account",
      date: "2026-11-14",
      description: "新建支付宝",
      accountPath: "Assets/支付宝",
      accountKind: "asset",
      openingBalance: "1000.00",
      warnings: [],
    });
    const onOpenChange = vi.fn();

    renderWithProviders(
      <AiInputDialog open={true} onOpenChange={onOpenChange} />,
    );

    fireEvent.change(screen.getByPlaceholderText(/中午吃饭|Lunch/i), {
      target: { value: "新建支付宝账户余额1000" },
    });
    fireEvent.click(screen.getByRole("button", { name: /识别并录入|Parse/i }));

    await waitFor(() => {
      expect(screen.getByText("确认记账")).toBeInTheDocument();
    });
    // Path and opening balance are prefilled and editable.
    expect(
      (screen.getByLabelText(/账户路径|Account path/i) as HTMLInputElement)
        .value,
    ).toBe("Assets/支付宝");

    // Confirming creates the account through the existing command.
    mockAccountCreate.mockResolvedValueOnce({ id: "acc-9" });
    fireEvent.click(screen.getByRole("button", { name: /保存|Save|确认/i }));
    await waitFor(() => {
      expect(mockAccountCreate).toHaveBeenCalledWith(
        expect.objectContaining({ name: "Assets/支付宝" }),
      );
    });
  });

  it("shows error on failure without closing dialog", async () => {
    mockAiParse.mockRejectedValueOnce(new Error("errors.ai.noApiKey"));
    const onOpenChange = vi.fn();

    renderWithProviders(
      <AiInputDialog open={true} onOpenChange={onOpenChange} />,
    );

    const input = screen.getByPlaceholderText(/中午吃饭|Lunch/i);
    fireEvent.change(input, { target: { value: "test" } });

    const button = screen.getByRole("button", { name: /识别并录入|Parse/i });
    fireEvent.click(button);

    await waitFor(() => {
      expect(screen.getByText(/errors\.ai\.noApiKey/i)).toBeInTheDocument();
    });

    expect(onOpenChange).not.toHaveBeenCalledWith(false);
  });

  it("disables submit button when text is empty", () => {
    renderWithProviders(<AiInputDialog open={true} onOpenChange={vi.fn()} />);
    const button = screen.getByRole("button", { name: /识别并录入|Parse/i });
    expect(button).toBeDisabled();
  });
});
