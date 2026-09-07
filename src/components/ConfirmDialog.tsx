import { type ReactNode, useState } from "react";
import { Modal } from "./Modal";

interface Props {
  title: string;
  message: ReactNode;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
  onConfirm: () => Promise<void> | void;
  onClose: () => void;
}

export function ConfirmDialog({
  title,
  message,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  danger,
  onConfirm,
  onClose,
}: Props) {
  const [busy, setBusy] = useState(false);

  const confirm = async () => {
    setBusy(true);
    try {
      await onConfirm();
      onClose();
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title={title}
      onClose={onClose}
      size="sm"
      footer={
        <>
          <button className="ck-btn ck-btn--ghost" onClick={onClose} disabled={busy}>
            {cancelLabel}
          </button>
          <button
            className={`ck-btn ${danger ? "ck-btn--danger" : "ck-btn--primary"}`}
            onClick={confirm}
            disabled={busy}
          >
            {confirmLabel}
          </button>
        </>
      }
    >
      <div className="ck-confirm-text">{message}</div>
    </Modal>
  );
}
