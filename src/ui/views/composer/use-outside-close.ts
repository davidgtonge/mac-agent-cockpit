import { useEffect } from "preact/hooks";

export function useOutsideClose(open: boolean, ref: { current: HTMLDivElement | null }, onClose: () => void) {
  useEffect(() => {
    if (!open) return;
    let removeListener: (() => void) | undefined;
    const timer = window.setTimeout(() => {
      const onPointerDown = (event: PointerEvent) => {
        if (!ref.current?.contains(event.target as Node)) onClose();
      };
      window.addEventListener("pointerdown", onPointerDown);
      removeListener = () => window.removeEventListener("pointerdown", onPointerDown);
    }, 0);
    return () => {
      window.clearTimeout(timer);
      removeListener?.();
    };
  }, [open, onClose, ref]);
}
