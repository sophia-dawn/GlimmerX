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
        console.log("[Shortcut] isRegistered:", alreadyRegistered);
        if (alreadyRegistered) {
          await unregister(shortcut);
          console.log("[Shortcut] unregistered existing");
        }
        if (mounted) {
          await register(shortcut, (event) => {
            console.log("[Shortcut] triggered:", event);
            callbackRef.current();
          });
          registeredRef.current = true;
          console.log("[Shortcut] registered:", shortcut);
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
        console.log("[Shortcut] cleanup:", shortcut);
      }
    };
  }, [shortcut, enabled]);
}
