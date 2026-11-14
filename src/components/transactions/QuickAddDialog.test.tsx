import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { QuickAddDialog } from "./QuickAddDialog";

vi.mock("@/utils/api", () => ({
  setUiLanguage: vi.fn().mockResolvedValue(undefined),
  accountList: vi.fn().mockResolvedValue([
    {
      id: "acc-cash",
      name: "Cash",
      account_type: "asset",
      is_active: true,
    },
    {
      id: "acc-wechat",
      name: "WeChat",
      account_type: "asset",
      is_active: true,
    },
  ]),
  categoryList: vi
    .fn()
    .mockResolvedValue([
      { id: "cat-food", name: "餐饮", type: "expense", icon: null },
    ]),
  quickAddTransaction: vi.fn(),
}));

vi.mock("@/utils/errorTranslation", () => ({
  translateErrorMessage: vi.fn((err) => String(err)),
}));

import { quickAddTransaction } from "@/utils/api";
const mockQuickAdd = quickAddTransaction as ReturnType<typeof vi.fn>;

function renderWithProviders(ui: React.ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
}

describe("QuickAddDialog review variant", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("prefills from the proposal and submits with categoryName", async () => {
    mockQuickAdd.mockResolvedValueOnce({ id: "tx-1" });
    const onOpenChange = vi.fn();

    renderWithProviders(
      <QuickAddDialog
        open={true}
        variant="review"
        onOpenChange={onOpenChange}
        initial={{
          mode: "expense",
          amount: "18.00",
          sourceAccountId: "acc-wechat",
          sourceAccountName: "WeChat",
          categoryName: "咖啡",
          description: "下午咖啡",
          date: "2026-11-14",
          warnings: ["分类「咖啡」是新建的"],
        }}
      />,
    );

    // Review title, not the blank-create title.
    expect(screen.getByText("确认记账")).toBeInTheDocument();

    // The reset effects must NOT wipe the prefill (design doc 25, P2-5).
    await waitFor(() => {
      const amount = screen.getByLabelText(/金额|Amount/) as HTMLInputElement;
      expect(amount.value).toBe("18.00");
    });
    const description = screen.getByLabelText(
      /备注|描述|Description/,
    ) as HTMLInputElement;
    expect(description.value).toBe("下午咖啡");

    // The proposed (not-yet-existing) category is visible as "new".
    expect(screen.getByText(/咖啡（新建）/)).toBeInTheDocument();

    // Submitting sends the proposed name via categoryName.
    fireEvent.click(screen.getByRole("button", { name: /保存|Save/i }));
    await waitFor(() => {
      expect(mockQuickAdd).toHaveBeenCalledWith(
        expect.objectContaining({
          mode: "expense",
          amount: "18.00",
          sourceAccountId: "acc-wechat",
          categoryName: "咖啡",
        }),
      );
    });
  });

  it("upgrades a proposed category to the existing id once loaded", async () => {
    renderWithProviders(
      <QuickAddDialog
        open={true}
        variant="review"
        onOpenChange={vi.fn()}
        initial={{
          mode: "expense",
          amount: "18.00",
          sourceAccountId: "acc-wechat",
          categoryName: "餐饮",
          date: "2026-11-14",
        }}
      />,
    );

    await waitFor(() => {
      expect(screen.queryByText(/餐饮（新建）/)).not.toBeInTheDocument();
    });
  });

  it("starts blank in the create variant", async () => {
    renderWithProviders(<QuickAddDialog open={true} onOpenChange={vi.fn()} />);
    expect(screen.getByText("快速记账")).toBeInTheDocument();
    await waitFor(() => {
      const amount = screen.getByLabelText(/金额|Amount/) as HTMLInputElement;
      expect(amount.value).toBe("");
    });
  });
});
