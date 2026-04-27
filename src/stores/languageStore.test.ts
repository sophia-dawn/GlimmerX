import { describe, it, expect, vi, beforeEach } from "vitest";
import { useLanguageStore } from "./languageStore";

vi.mock("@/i18n", () => ({
  setLanguage: vi.fn(),
  getCurrentLanguage: vi.fn(() => "zh"),
}));

describe("languageStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("initializes with current language", () => {
    const store = useLanguageStore.getState();
    expect(store.language).toBe("zh");
  });

  it("changes language", () => {
    useLanguageStore.getState().setLanguage("en");
    expect(useLanguageStore.getState().language).toBe("en");
  });

  it("changes back to Chinese", () => {
    useLanguageStore.getState().setLanguage("zh");
    expect(useLanguageStore.getState().language).toBe("zh");
  });
});
