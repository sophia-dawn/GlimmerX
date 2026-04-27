import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { ChangePasswordDialog } from "./ChangePasswordDialog";
import { dbChangePassword, dbLock } from "@/utils/api";
import { toast } from "sonner";
import { useNavigate } from "react-router-dom";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("react-router-dom", () => ({
  useNavigate: vi.fn(),
}));

vi.mock("@/utils/api", () => ({
  dbChangePassword: vi.fn(),
  dbLock: vi.fn(),
}));

vi.mock("@/utils/errorTranslation", () => ({
  translateErrorMessage: vi.fn((err) => String(err)),
}));

vi.mock("@/stores/dbStore", () => ({
  useDbStore: () => ({ setLocked: vi.fn() }),
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

describe("ChangePasswordDialog", () => {
  const defaultProps = {
    open: true,
    onOpenChange: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders dialog with title and description", () => {
    render(<ChangePasswordDialog {...defaultProps} />);
    expect(
      screen.getByText("settings.changePasswordTitle"),
    ).toBeInTheDocument();
    expect(screen.getByText("settings.changePasswordDesc")).toBeInTheDocument();
  });

  it("shows all password input fields", () => {
    render(<ChangePasswordDialog {...defaultProps} />);
    expect(screen.getByLabelText("settings.oldPassword")).toBeInTheDocument();
    expect(screen.getByLabelText("settings.newPassword")).toBeInTheDocument();
    expect(
      screen.getByLabelText("settings.confirmNewPassword"),
    ).toBeInTheDocument();
  });

  it("shows cancel and submit buttons", () => {
    render(<ChangePasswordDialog {...defaultProps} />);
    expect(
      screen.getByRole("button", { name: "common.cancel" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "settings.changePassword" }),
    ).toBeInTheDocument();
  });

  it("calls onOpenChange(false) when cancel is clicked", () => {
    const onOpenChange = vi.fn();
    render(
      <ChangePasswordDialog {...defaultProps} onOpenChange={onOpenChange} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "common.cancel" }));
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it("shows error when passwords do not match", async () => {
    render(<ChangePasswordDialog {...defaultProps} />);
    const oldInput = screen.getByLabelText("settings.oldPassword");
    const newInput = screen.getByLabelText("settings.newPassword");
    const confirmInput = screen.getByLabelText("settings.confirmNewPassword");

    fireEvent.change(oldInput, { target: { value: "old123" } });
    fireEvent.change(newInput, { target: { value: "new123" } });
    fireEvent.change(confirmInput, { target: { value: "different" } });

    fireEvent.click(
      screen.getByRole("button", { name: "settings.changePassword" }),
    );

    await waitFor(() => {
      expect(
        screen.getByText("unlock.passwordsDoNotMatch"),
      ).toBeInTheDocument();
    });
  });

  it("shows error when newPassword is too short", async () => {
    render(<ChangePasswordDialog {...defaultProps} />);
    const newInput = screen.getByLabelText("settings.newPassword");
    fireEvent.change(newInput, { target: { value: "ab" } });
    fireEvent.click(
      screen.getByRole("button", { name: "settings.changePassword" }),
    );
    await waitFor(() => {
      expect(screen.getByText("unlock.passwordMinLength")).toBeInTheDocument();
    });
  });

  it("shows error when fields are empty", async () => {
    render(<ChangePasswordDialog {...defaultProps} />);
    fireEvent.click(
      screen.getByRole("button", { name: "settings.changePassword" }),
    );
    await waitFor(() => {
      // oldPassword and newPassword show passwordRequired, confirmNewPassword shows confirmPasswordRequired
      expect(screen.getAllByText("unlock.passwordRequired").length).toBe(2);
      expect(
        screen.getByText("unlock.confirmPasswordRequired"),
      ).toBeInTheDocument();
    });
  });

  it("calls API and shows success on valid submission", async () => {
    vi.mocked(dbChangePassword).mockResolvedValueOnce(undefined);
    vi.mocked(dbLock).mockResolvedValueOnce(undefined);
    const mockNavigate = vi.fn();
    vi.mocked(useNavigate).mockReturnValue(mockNavigate);

    render(<ChangePasswordDialog {...defaultProps} />);

    fireEvent.change(screen.getByLabelText("settings.oldPassword"), {
      target: { value: "old123" },
    });
    fireEvent.change(screen.getByLabelText("settings.newPassword"), {
      target: { value: "new123" },
    });
    fireEvent.change(screen.getByLabelText("settings.confirmNewPassword"), {
      target: { value: "new123" },
    });

    fireEvent.click(
      screen.getByRole("button", { name: "settings.changePassword" }),
    );

    await waitFor(() => {
      expect(dbChangePassword).toHaveBeenCalledWith("old123", "new123");
      expect(dbLock).toHaveBeenCalled();
      expect(mockNavigate).toHaveBeenCalledWith("/unlock");
    });
    expect(vi.mocked(toast.success)).toHaveBeenCalled();
  });

  it("shows error toast when API fails", async () => {
    vi.mocked(dbChangePassword).mockRejectedValueOnce(
      new Error("Invalid password"),
    );

    render(<ChangePasswordDialog {...defaultProps} />);

    fireEvent.change(screen.getByLabelText("settings.oldPassword"), {
      target: { value: "wrong" },
    });
    fireEvent.change(screen.getByLabelText("settings.newPassword"), {
      target: { value: "new123" },
    });
    fireEvent.change(screen.getByLabelText("settings.confirmNewPassword"), {
      target: { value: "new123" },
    });

    fireEvent.click(
      screen.getByRole("button", { name: "settings.changePassword" }),
    );

    await waitFor(() => {
      expect(vi.mocked(toast.error)).toHaveBeenCalled();
    });
  });
});
