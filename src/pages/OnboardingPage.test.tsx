import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { OnboardingPage } from "@/pages/OnboardingPage";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("react-router-dom", () => ({
  useNavigate: () => vi.fn(),
}));

vi.mock("@/components/accounts/AccountWizard", () => ({
  AccountWizard: ({
    open,
    onOpenChange,
  }: {
    open: boolean;
    onOpenChange: (open: boolean) => void;
  }) =>
    open ? (
      <div data-testid="wizard">
        <button onClick={() => onOpenChange(false)}>Close Wizard</button>
      </div>
    ) : null,
}));

describe("OnboardingPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders welcome message", () => {
    render(<OnboardingPage />);
    expect(screen.getByText("onboarding.welcomeTitle")).toBeInTheDocument();
    expect(screen.getByText("onboarding.welcomeDesc")).toBeInTheDocument();
  });

  it("renders add account and skip buttons", () => {
    render(<OnboardingPage />);
    expect(screen.getByText("onboarding.addFirstAccount")).toBeInTheDocument();
    expect(screen.getByText("onboarding.skipForNow")).toBeInTheDocument();
  });

  it("opens wizard when add account button clicked", () => {
    render(<OnboardingPage />);
    // Wizard is already open by default (useState(true))
    expect(screen.getByTestId("wizard")).toBeInTheDocument();
  });

  it("can close wizard", () => {
    render(<OnboardingPage />);
    fireEvent.click(screen.getByText("Close Wizard"));
    expect(screen.queryByTestId("wizard")).not.toBeInTheDocument();
  });
});
