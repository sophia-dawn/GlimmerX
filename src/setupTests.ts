import { vi } from "vitest";
import "@testing-library/jest-dom";
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "@/i18n/locales/en.json";
import zh from "@/i18n/locales/zh.json";

// Mock scrollIntoView (not available in jsdom, needed by Radix UI)
Element.prototype.scrollIntoView = vi.fn();
Element.prototype.scrollTo = vi.fn();

// Mock localStorage with proper storage functionality
const localStorageStore: Record<string, string> = {};
Object.defineProperty(window, "localStorage", {
  value: {
    getItem: (key: string) => localStorageStore[key] ?? null,
    setItem: (key: string, value: string) => {
      localStorageStore[key] = value;
    },
    removeItem: (key: string) => {
      delete localStorageStore[key];
    },
    clear: () => {
      for (const key of Object.keys(localStorageStore)) {
        delete localStorageStore[key];
      }
    },
  },
  writable: true,
});

// Mock navigator.language
Object.defineProperty(window.navigator, "language", {
  value: "zh-CN",
  writable: true,
});

// Mock window.matchMedia (not available in jsdom, needed by themeStore)
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  })),
});

// Global mock for react-i18next - provides complete interface
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) => {
      const keys = key.split(".");
      let value: unknown = zh;
      for (const k of keys) {
        value = (value as Record<string, unknown>)?.[k];
      }
      if (typeof value === "string") {
        if (options) {
          return Object.entries(options).reduce(
            (str, [k, v]) => str.replace(new RegExp(`{{${k}}}`, "g"), String(v)),
            value,
          );
        }
        return value;
      }
      return key;
    },
    i18n: {
      language: "zh",
      changeLanguage: vi.fn(),
    },
  }),
  initReactI18next: {
    type: "3rdParty",
    init: vi.fn(),
  },
}));

// Initialize i18n for tests
i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    zh: { translation: zh },
  },
  lng: "zh",
  fallbackLng: "en",
  interpolation: {
    escapeValue: false,
  },
});
