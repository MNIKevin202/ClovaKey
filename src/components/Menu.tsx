import { type ReactNode, useEffect, useId, useRef, useState } from "react";

export type MenuItem =
  | {
      label: string;
      icon?: ReactNode;
      onClick: () => void;
      danger?: boolean;
      disabled?: boolean;
    }
  | { separator: true };

interface Props {
  trigger: (props: {
    onClick: (e: React.MouseEvent) => void;
    "aria-expanded": boolean;
    id: string;
  }) => ReactNode;
  items: MenuItem[];
  align?: "left" | "right";
}

/** A small accessible dropdown menu with click-outside + Escape handling. */
export function Menu({ trigger, items, align = "right" }: Props) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const id = useId();

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div className="ck-menu" ref={rootRef}>
      {trigger({
        id,
        "aria-expanded": open,
        onClick: (e) => {
          e.stopPropagation();
          setOpen((o) => !o);
        },
      })}
      {open && (
        <div className={`ck-menu__list ck-menu__list--${align}`} role="menu" aria-labelledby={id}>
          {items.map((item, i) =>
            "separator" in item ? (
              <div key={i} className="ck-menu__sep" role="separator" />
            ) : (
              <button
                key={i}
                role="menuitem"
                className={`ck-menu__item${item.danger ? " is-danger" : ""}`}
                disabled={item.disabled}
                onClick={(e) => {
                  e.stopPropagation();
                  setOpen(false);
                  item.onClick();
                }}
              >
                {item.icon && <span className="ck-menu__icon">{item.icon}</span>}
                {item.label}
              </button>
            ),
          )}
        </div>
      )}
    </div>
  );
}
