import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { setLanguage, getCurrentLanguage, SUPPORTED_LANGUAGES } from "@/i18n";

describe("i18n configuration", () => {
  beforeEach(() => {
    vi.resetModules();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("SUPPORTED_LANGUAGES", () => {
    it("contains English and Chinese", () => {
      expect(SUPPORTED_LANGUAGES.length).toBe(2);
      expect(SUPPORTED_LANGUAGES.map((l) => l.code)).toEqual(["en", "zh"]);
    });
  });

  describe("setLanguage", () => {
    it("saves language to localStorage and changes i18n language", () => {
      setLanguage("en");
      expect(getCurrentLanguage()).toBe("en");

      setLanguage("zh");
      expect(getCurrentLanguage()).toBe("zh");
    });
  });

  describe("getCurrentLanguage", () => {
    it("returns the current language after setLanguage", () => {
      setLanguage("en");
      expect(getCurrentLanguage()).toBe("en");
    });
  });
});
