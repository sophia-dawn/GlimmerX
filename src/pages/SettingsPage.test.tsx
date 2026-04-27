import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { SettingsPage } from "@/pages/SettingsPage";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("react-router-dom", () => ({
  useNavigate: () => vi.fn(),
}));

const mockSetMode = vi.fn();
const mockSetColor = vi.fn();
const mockSetAtmosphere = vi.fn();
const mockSetLanguage = vi.fn();

vi.mock("@/stores/themeStore", () => ({
  useThemeStore: () => ({
    mode: "light",
    color: "neutral",
    atmosphere: "none",
    setMode: mockSetMode,
    setColor: mockSetColor,
    setAtmosphere: mockSetAtmosphere,
  }),
}));

vi.mock("@/stores/languageStore", () => ({
  useLanguageStore: () => ({
    language: "zh",
    setLanguage: mockSetLanguage,
  }),
}));

describe("SettingsPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders theme, color, and language sections", () => {
    render(<SettingsPage />);
    expect(screen.getByText("settings.theme")).toBeInTheDocument();
    expect(screen.getByText("settings.color")).toBeInTheDocument();
    expect(screen.getByText("settings.language")).toBeInTheDocument();
  });

  it("changes theme when theme button clicked", () => {
    render(<SettingsPage />);
    const darkButton = screen.getByRole("button", {
      name: /navigation\.themeDark/i,
    });
    fireEvent.click(darkButton);
    expect(mockSetMode).toHaveBeenCalledWith("dark");
  });

  it("changes color when color button clicked", () => {
    render(<SettingsPage />);
    const blueButton = screen.getByRole("button", {
      name: /settings\.colorBlue/i,
    });
    fireEvent.click(blueButton);
    expect(mockSetColor).toHaveBeenCalledWith("blue");
  });

  it("changes language when select changed", () => {
    render(<SettingsPage />);
    const selectTrigger = screen.getByRole("combobox");
    fireEvent.click(selectTrigger);
    // Select items might be in a portal, check by text
    fireEvent.click(screen.getByText("English"));
    expect(mockSetLanguage).toHaveBeenCalledWith("en");
  });

  describe("Atmosphere selection", () => {
    beforeEach(() => {
      localStorage.clear();
    });

    it("renders atmosphere options", () => {
      render(<SettingsPage />);

      expect(screen.getByText("settings.atmosphereNone")).toBeInTheDocument();
      expect(screen.getByText("settings.atmosphereWarm")).toBeInTheDocument();
      expect(screen.getByText("settings.atmosphereOcean")).toBeInTheDocument();
    });

    it("calls setAtmosphere when atmosphere button clicked", () => {
      render(<SettingsPage />);

      const warmButton = screen
        .getByText("settings.atmosphereWarm")
        .closest("button");
      fireEvent.click(warmButton!);

      expect(mockSetAtmosphere).toHaveBeenCalledWith("warm");
    });
  });
});
