import { useEffect, useRef } from "react";
import {
  register,
  unregister,
  isRegistered,
} from "@tauri-apps/plugin-global-shortcut";

export function useGlobalShortcut(
  shortcut: string,
  callback: () => void,
  enabled: boolean = true,
) {
  const callbackRef = useRef(callback);
  callbackRef.current = callback;
  const registeredRef = useRef(false);

  useEffect(() => {
    if (!enabled) {
      return;
    }

    let mounted = true;

    const registerShortcut = async () => {
      try {
        const alreadyRegistered = await isRegistered(shortcut);
        if (alreadyRegistered) {
          await unregister(shortcut);
        }
        if (mounted) {
          await register(shortcut, () => {
            callbackRef.current();
          });
          registeredRef.current = true;
        }
      } catch (e) {
        console.error("[Shortcut] Failed to register:", e);
      }
    };

    registerShortcut();

    return () => {
      mounted = false;
      if (registeredRef.current) {
        unregister(shortcut).catch(() => {});
        registeredRef.current = false;
      }
    };
  }, [shortcut, enabled]);
}
