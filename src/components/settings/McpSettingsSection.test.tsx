import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { McpSettingsSection } from "./McpSettingsSection";

vi.mock("@/utils/api", () => ({
  setUiLanguage: vi.fn().mockResolvedValue(undefined),
  mcpGetConfig: vi.fn(),
  mcpSetConfig: vi.fn(),
  mcpResetToken: vi.fn(),
}));

vi.mock("@/utils/errorTranslation", () => ({
  translateErrorMessage: vi.fn((err) => String(err)),
}));

import { mcpGetConfig, mcpSetConfig, mcpResetToken } from "@/utils/api";

const mockGet = mcpGetConfig as ReturnType<typeof vi.fn>;
const mockSet = mcpSetConfig as ReturnType<typeof vi.fn>;
const mockReset = mcpResetToken as ReturnType<typeof vi.fn>;

const baseConfig = {
  enabled: false,
  port: 17823,
  actualPort: null,
  running: false,
  trustMode: "confirm",
  token: "a".repeat(64),
};

describe("McpSettingsSection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGet.mockResolvedValue({ ...baseConfig });
  });

  it("shows the disabled state with the stored token", async () => {
    render(<McpSettingsSection />);
    await waitFor(() => {
      expect(screen.getByDisplayValue("a".repeat(64))).toBeInTheDocument();
    });
    expect(screen.getByText(/未运行|Not running/)).toBeInTheDocument();
    // The client config block embeds the token and the preferred port.
    expect(screen.getByText(/127\.0\.0\.1:17823\/mcp/)).toBeInTheDocument();
  });

  it("flipping the switch persists enabled and reflects the running state", async () => {
    mockSet.mockResolvedValue({
      ...baseConfig,
      enabled: true,
      running: true,
      actualPort: 17823,
    });
    render(<McpSettingsSection />);
    await screen.findByDisplayValue("a".repeat(64));

    const toggle = screen.getByRole("switch");
    // Radix Switch needs the pointer lifecycle in jsdom, a bare click is a
    // no-op on it.
    fireEvent.pointerDown(toggle);
    fireEvent.click(toggle);

    await waitFor(() => {
      expect(mockSet).toHaveBeenCalledWith({ enabled: true });
    });
    await waitFor(() => {
      expect(
        screen.getByText(/运行中 · 127\.0\.0\.1:17823/),
      ).toBeInTheDocument();
    });
  });

  it("changing the trust mode persists it", async () => {
    mockSet.mockResolvedValue({ ...baseConfig });
    render(<McpSettingsSection />);
    await screen.findByDisplayValue("a".repeat(64));

    // Open the trust select (shadcn Select) and pick the read-only tier.
    fireEvent.click(screen.getByRole("combobox"));
    const option = await screen.findByRole("option", {
      name: /只读|Read-only/,
    });
    fireEvent.click(option);

    await waitFor(() => {
      expect(mockSet).toHaveBeenCalledWith({ trust: "readonly" });
    });
  });

  it("resetting the token rotates the shown value", async () => {
    mockReset.mockResolvedValue({
      ...baseConfig,
      token: "b".repeat(64),
    });
    render(<McpSettingsSection />);
    await screen.findByDisplayValue("a".repeat(64));

    const reset = screen.getByTitle(/重置令牌|Reset token/);
    fireEvent.click(reset);

    await waitFor(() => {
      expect(mockReset).toHaveBeenCalled();
    });
    await waitFor(() => {
      expect(screen.getByDisplayValue("b".repeat(64))).toBeInTheDocument();
    });
  });
});
