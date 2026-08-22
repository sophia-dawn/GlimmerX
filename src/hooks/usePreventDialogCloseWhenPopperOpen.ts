import { useEffect, useRef } from "react";

const POPPER_OPEN_SELECTOR =
  '[data-slot="select-content"][data-state="open"], [data-slot="popover-content"][data-state="open"]';

export function usePreventDialogCloseWhenPopperOpen(open: boolean) {
  const popperOpenAtPointerDownRef = useRef(false);

  useEffect(() => {
    if (!open) {
      popperOpenAtPointerDownRef.current = false;
      return;
    }
    const handlePointerDown = () => {
      popperOpenAtPointerDownRef.current =
        !!document.querySelector(POPPER_OPEN_SELECTOR);
    };
    document.addEventListener("pointerdown", handlePointerDown, true);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown, true);
    };
  }, [open]);

  const onInteractOutside = (e: Event) => {
    if (popperOpenAtPointerDownRef.current) {
      e.preventDefault();
      popperOpenAtPointerDownRef.current = false;
      return;
    }
    if ("detail" in e) {
      const event = (e as CustomEvent).detail?.originalEvent as
        PointerEvent | undefined;
      if (event?.target instanceof Element) {
        if (event.target.closest(POPPER_OPEN_SELECTOR)) {
          e.preventDefault();
        }
      }
    }
  };

  return onInteractOutside;
}
