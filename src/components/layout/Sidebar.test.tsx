import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("react-router-dom", () => ({
  useNavigate: () => vi.fn(),
  useLocation: () => ({ pathname: "/categories" }),
  NavLink: ({
    to,
    children,
    className,
  }: {
    to: string;
    children: React.ReactNode;
    className?: string | ((props: { isActive: boolean }) => string);
  }) => {
    const isActive = to === "/categories";
    const cls =
      typeof className === "function" ? className({ isActive }) : className;
    return (
      <a href={to} data-active={String(isActive)} className={cls}>
        {children}
      </a>
    );
  },
}));

vi.mock("@/stores/dbStore", () => ({
  useDbStore: () => ({ setLocked: vi.fn() }),
}));

vi.mock("@/utils/api", () => ({
  dbLock: vi.fn().mockResolvedValue(undefined),
}));

import { Sidebar } from "@/components/layout/Sidebar";

describe("Sidebar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the logo", () => {
    render(<Sidebar />);
    expect(screen.getByText("GlimmerX")).toBeInTheDocument();
  });

  it("renders navigation links", () => {
    render(<Sidebar />);
    expect(screen.getByText("navigation.overview")).toBeInTheDocument();
    expect(screen.getByText("navigation.accounts")).toBeInTheDocument();
    expect(screen.getByText("navigation.transactions")).toBeInTheDocument();
    expect(screen.getByText("navigation.categories")).toBeInTheDocument();
    expect(screen.getByText("navigation.budgets")).toBeInTheDocument();
    expect(screen.getByText("navigation.reports")).toBeInTheDocument();
    expect(screen.getByText("navigation.settings")).toBeInTheDocument();
  });

  it("renders lock and switch ledger buttons", () => {
    render(<Sidebar />);
    expect(screen.getByText("navigation.lockScreen")).toBeInTheDocument();
    expect(screen.getByText("navigation.switchLedger")).toBeInTheDocument();
  });

  it("marks the settings link as inactive for other routes", () => {
    render(<Sidebar />);
    const settingsLink = screen.getByRole("link", {
      name: /navigation\.settings/,
    });
    expect(settingsLink).toHaveAttribute("data-active", "false");
  });

  it("marks the categories link as active", () => {
    render(<Sidebar />);
    const categoriesLink = screen.getByRole("link", {
      name: /navigation\.categories/,
    });
    expect(categoriesLink).toHaveAttribute("data-active", "true");
  });
});
