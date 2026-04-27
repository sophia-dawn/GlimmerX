import { Toaster } from "sonner";
import { useThemeStore } from "@/stores/themeStore";

export function ToasterProvider() {
  const { effectiveTheme } = useThemeStore();

  return (
    <Toaster
      position="bottom-right"
      duration={3000}
      theme={effectiveTheme}
      richColors
      closeButton
    />
  );
}
