import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { AppShell } from "@/components/layout/AppShell";

vi.mock("react-router-dom", () => ({
  Outlet: () => <div data-testid="outlet" />,
}));

vi.mock("./Sidebar", () => ({
  Sidebar: () => <div data-testid="sidebar" />,
}));

vi.mock("./Header", () => ({
  Header: () => <div data-testid="header" />,
}));

describe("AppShell", () => {
  it("renders sidebar, header, and outlet", () => {
    render(<AppShell />);
    expect(screen.getByTestId("sidebar")).toBeInTheDocument();
    expect(screen.getByTestId("header")).toBeInTheDocument();
    expect(screen.getByTestId("outlet")).toBeInTheDocument();
  });

  it("has correct layout structure", () => {
    const { container } = render(<AppShell />);
    const main = container.querySelector("main");
    expect(main).toBeInTheDocument();
  });
});
