import type { ReactNode } from "react";

interface Props {
  icon?: ReactNode;
  title: string;
  message: string;
  actions?: ReactNode;
}

export function EmptyState({ icon, title, message, actions }: Props) {
  return (
    <div className="ck-empty">
      {icon && <div className="ck-empty__icon">{icon}</div>}
      <h2 className="ck-empty__title">{title}</h2>
      <p className="ck-empty__message">{message}</p>
      {actions && <div className="ck-empty__actions">{actions}</div>}
    </div>
  );
}
