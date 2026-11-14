import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  render,
  screen,
  waitFor,
  fireEvent,
  act,
} from "@testing-library/react";
import { McpApprovalDialog } from "./McpApprovalDialog";

// listen must exist: the dialog registers the backend event handler.
type Listener = (event: { payload: unknown }) => void;
let capturedListener: Listener | null = null;

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockImplementation((_name: string, handler: Listener) => {
    capturedListener = handler;
    return Promise.resolve(() => {
      capturedListener = null;
    });
  }),
}));

vi.mock("@/utils/api", () => ({
  setUiLanguage: vi.fn().mockResolvedValue(undefined),
  mcpCurrentApproval: vi.fn().mockResolvedValue(null),
  mcpApproveRequest: vi.fn().mockResolvedValue(true),
}));

vi.mock("@/utils/errorTranslation", () => ({
  translateErrorMessage: vi.fn((err) => String(err)),
}));

import { mcpApproveRequest, mcpCurrentApproval } from "@/utils/api";

const mockApprove = mcpApproveRequest as ReturnType<typeof vi.fn>;

const transactionRequest = {
  requestId: "req-1",
  tool: "record_transaction",
  request: {
    kind: "expense",
    amountCents: 1850,
    date: "2026-11-14",
    description: "lunch",
    sourceAccount: "WeChat",
    destinationAccount: null,
    category: "餐饮",
  },
  createdAt: "2026-11-14T12:00:00Z",
};

describe("McpApprovalDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    capturedListener = null;
    (mcpCurrentApproval as ReturnType<typeof vi.fn>).mockResolvedValue(null);
  });

  it("stays closed without a pending approval", () => {
    render(<McpApprovalDialog />);
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("shows the request verbatim when the event fires", async () => {
    render(<McpApprovalDialog />);
    expect(capturedListener).not.toBeNull();
    act(() => {
      capturedListener!({ payload: transactionRequest });
    });

    await waitFor(() => {
      expect(screen.getByText(/¥18\.50/)).toBeInTheDocument();
    });
    // Account, category and description exactly as the agent sent them.
    expect(screen.getByText("WeChat")).toBeInTheDocument();
    expect(screen.getByText("餐饮")).toBeInTheDocument();
    expect(screen.getByText("lunch")).toBeInTheDocument();
  });

  it("approving resolves the request id with true", async () => {
    render(<McpApprovalDialog />);
    act(() => {
      capturedListener!({ payload: transactionRequest });
    });

    const approve = await screen.findByRole("button", {
      name: /确认写入|Approve/i,
    });
    fireEvent.click(approve);

    await waitFor(() => {
      expect(mockApprove).toHaveBeenCalledWith("req-1", true);
    });
  });

  it("rejecting resolves the request id with false", async () => {
    render(<McpApprovalDialog />);
    act(() => {
      capturedListener!({ payload: transactionRequest });
    });

    const reject = await screen.findByRole("button", { name: /拒绝|Reject/i });
    fireEvent.click(reject);

    await waitFor(() => {
      expect(mockApprove).toHaveBeenCalledWith("req-1", false);
    });
  });

  it("renders create_account requests with the path and opening balance", async () => {
    render(<McpApprovalDialog />);
    act(() => {
      capturedListener!({
        payload: {
          requestId: "req-2",
          tool: "create_account",
          request: {
            path: "Assets/Alipay",
            openingBalanceCents: 10000,
          },
          createdAt: "2026-11-14T12:00:00Z",
        },
      });
    });

    await waitFor(() => {
      expect(screen.getByText("Assets/Alipay")).toBeInTheDocument();
    });
    expect(screen.getByText(/¥100\.00/)).toBeInTheDocument();
  });
});
