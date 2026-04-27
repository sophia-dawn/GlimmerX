import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { PlaceholderPage } from "@/components/layout/PlaceholderPage";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, vars?: Record<string, unknown>) => {
      if (vars?.title) return `in-development: ${vars.title}`;
      return key;
    },
  }),
}));

describe("PlaceholderPage", () => {
  it("renders the title in the development message", () => {
    render(<PlaceholderPage title="Budgets" />);
    expect(screen.getByText(/in-development: Budgets/)).toBeInTheDocument();
  });

  it("centers content vertically and horizontally", () => {
    const { container } = render(<PlaceholderPage title="X" />);
    const wrapper = container.firstChild as HTMLElement;
    expect(wrapper).toHaveClass("flex", "items-center", "justify-center");
  });
});
