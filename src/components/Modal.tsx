import { type ReactNode, useEffect, useRef } from "react";
import { X } from "lucide-react";

interface Props {
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  size?: "sm" | "md" | "lg";
  subtitle?: string;
}

/** Base modal dialog: overlay, Escape to close, initial focus, focus trap. */
export function Modal({ title, onClose, children, footer, size = "md", subtitle }: Props) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
      if (e.key === "Tab" && ref.current) {
        const focusable = ref.current.querySelectorAll<HTMLElement>(
          'a[href], button:not([disabled]), textarea, input, select, [tabindex]:not([tabindex="-1"])',
        );
        if (focusable.length === 0) return;
        const first = focusable[0];
        const last = focusable[focusable.length - 1];
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };
    document.addEventListener("keydown", onKey, true);
    // Focus the first meaningful control.
    const t = window.setTimeout(() => {
      const el = ref.current?.querySelector<HTMLElement>(
        "input, textarea, select, button:not(.ck-iconbtn)",
      );
      el?.focus();
    }, 30);
    return () => {
      document.removeEventListener("keydown", onKey, true);
      window.clearTimeout(t);
    };
  }, [onClose]);

  return (
    <div className="ck-overlay" onMouseDown={onClose}>
      <div
        ref={ref}
        className={`ck-modal ck-modal--${size}`}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        onMouseDown={(e) => e.stopPropagation()}
      >
        <header className="ck-modal__head">
          <div>
            <h2 className="ck-modal__title">{title}</h2>
            {subtitle && <p className="ck-modal__subtitle">{subtitle}</p>}
          </div>
          <button className="ck-iconbtn" aria-label="Close" onClick={onClose}>
            <X size={18} />
          </button>
        </header>
        <div className="ck-modal__body">{children}</div>
        {footer && <footer className="ck-modal__foot">{footer}</footer>}
      </div>
    </div>
  );
}
