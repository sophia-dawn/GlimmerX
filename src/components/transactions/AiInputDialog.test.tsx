import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AiInputDialog } from "./AiInputDialog";

// Mock the API module
vi.mock("@/utils/api", () => ({
  aiParseTransaction: vi.fn(),
}));

// Mock error translation
vi.mock("@/utils/errorTranslation", () => ({
  translateErrorMessage: vi.fn((err) => String(err)),
}));

import { aiParseTransaction } from "@/utils/api";

const mockAiParse = aiParseTransaction as ReturnType<typeof vi.fn>;

function renderWithProviders(ui: React.ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>,
  );
}

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

  it("submits text and closes dialog on success", async () => {
    mockAiParse.mockResolvedValueOnce({ id: "tx-1" });
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

    await waitFor(() => {
      expect(onOpenChange).toHaveBeenCalledWith(false);
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
