import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("react-router-dom", () => ({
  useLocation: () => ({ pathname: "/accounts" }),
}));

import { Header } from "@/components/layout/Header";
import { HeaderProvider } from "@/contexts/HeaderContext";

describe("Header", () => {
  it("renders the header title", () => {
    render(
      <HeaderProvider>
        <Header />
      </HeaderProvider>,
    );
    expect(screen.getByText("navigation.accounts")).toBeInTheDocument();
  });
});
