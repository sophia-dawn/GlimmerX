import { useEffect, useRef } from "react";
import {
  register,
  unregister,
  isRegistered,
} from "@tauri-apps/plugin-global-shortcut";

/**
 * Register a system-wide accelerator for as long as the component is mounted.
 *
 * @param onError called when registration fails — usually because another
 *   application already owns the combination. Silence here is a bug: the user
 *   sets a shortcut, nothing happens, and there is no way to find out why.
 */
export function useGlobalShortcut(
  shortcut: string,
  callback: () => void,
  enabled: boolean = true,
  onError?: (error: unknown) => void,
) {
  const callbackRef = useRef(callback);
  callbackRef.current = callback;
  const onErrorRef = useRef(onError);
  onErrorRef.current = onError;
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
        onErrorRef.current?.(e);
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
