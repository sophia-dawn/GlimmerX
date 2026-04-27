import { create } from "zustand";
import type { SupportedLanguage } from "@/i18n";
import { setLanguage, getCurrentLanguage } from "@/i18n";

interface LanguageState {
  language: SupportedLanguage;
  setLanguage: (lng: SupportedLanguage) => void;
}

export const useLanguageStore = create<LanguageState>((set) => ({
  language: getCurrentLanguage(),
  setLanguage: (lng) => {
    setLanguage(lng);
    set({ language: lng });
  },
}));
