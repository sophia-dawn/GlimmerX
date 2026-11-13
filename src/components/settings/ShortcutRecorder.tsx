import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Keyboard } from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { acceleratorFromEvent, displayAccelerator } from "@/utils/shortcut";

interface ShortcutRecorderProps {
  value: string;
  onChange: (accelerator: string) => void;
  onReset?: () => void;
}

/**
 * Click, then press the combination.
 *
 * Listens in the capture phase and swallows the event so the keystroke does not
 * also reach the app's own shortcut handlers — otherwise recording a new binding
 * would immediately trigger the old one.
 */
export function ShortcutRecorder({
  value,
  onChange,
  onReset,
}: ShortcutRecorderProps) {
  const { t } = useTranslation();
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    if (!recording) return;

    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();

      if (event.key === "Escape") {
        setRecording(false);
        return;
      }

      const accelerator = acceleratorFromEvent(event);
      // Modifier-only or modifier-less presses are not usable globally; keep
      // waiting instead of binding something surprising.
      if (!accelerator) return;

      onChange(accelerator);
      setRecording(false);
    };

    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [recording, onChange]);

  return (
    <div className="flex items-center gap-2">
      <Button
        type="button"
        variant="outline"
        onClick={() => setRecording((prev) => !prev)}
        className={cn(
          "min-w-[180px] justify-start gap-2 font-mono text-sm",
          recording && "border-primary text-primary",
        )}
      >
        <Keyboard className="h-4 w-4" />
        {recording
          ? t("settings.shortcutRecording")
          : displayAccelerator(value)}
      </Button>
      {onReset && (
        <Button type="button" variant="ghost" size="sm" onClick={onReset}>
          {t("settings.shortcutReset")}
        </Button>
      )}
    </div>
  );
}
