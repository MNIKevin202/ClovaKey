import { useState } from "react";
import { AlertTriangle } from "lucide-react";
import { Modal } from "@/components/Modal";
import { useStore } from "@/state/store";
import { errorMessage } from "@/lib/types";
import * as ipc from "@/lib/ipc";

const CONFIRM_WORD = "DELETE";

export function WipeDataDialog({ onClose }: { onClose: () => void }) {
  const protection = useStore((s) => s.status?.protection);
  const refreshStatus = useStore((s) => s.refreshStatus);
  const showToast = useStore((s) => s.showToast);
  const [passphrase, setPassphrase] = useState("");
  const [typed, setTyped] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const needsPass = protection === "passphrase";
  const valid = needsPass ? passphrase.length > 0 : typed === CONFIRM_WORD;

  const wipe = async () => {
    if (!valid) return;
    setBusy(true);
    setError(null);
    try {
      await ipc.dataWipe(needsPass ? passphrase : null);
      await refreshStatus();
      useStore.setState({ accounts: [], groups: [], codes: {} });
      showToast("All data erased");
      onClose();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title="Erase all ClovaKey data"
      onClose={onClose}
      size="sm"
      footer={
        <>
          <button className="ck-btn ck-btn--ghost" onClick={onClose} disabled={busy}>
            Cancel
          </button>
          <button className="ck-btn ck-btn--danger" onClick={wipe} disabled={!valid || busy}>
            Erase everything
          </button>
        </>
      }
    >
      <div className="ck-form">
        <div className="ck-callout ck-callout--danger">
          <AlertTriangle size={18} />
          <div>
            This permanently deletes <strong>every account, group, and setting</strong> and removes
            the vault key from your keychain. Make sure you have another way to sign in to your
            services, or an up-to-date backup. This cannot be undone.
          </div>
        </div>
        {needsPass ? (
          <label className="ck-field">
            <span className="ck-field__label">Enter your passphrase to confirm</span>
            <input
              className="ck-input"
              type="password"
              value={passphrase}
              onChange={(e) => setPassphrase(e.target.value)}
              autoFocus
            />
          </label>
        ) : (
          <label className="ck-field">
            <span className="ck-field__label">
              Type <strong>{CONFIRM_WORD}</strong> to confirm
            </span>
            <input
              className="ck-input"
              value={typed}
              onChange={(e) => setTyped(e.target.value)}
              autoFocus
              autoComplete="off"
            />
          </label>
        )}
        {error && <p className="ck-note ck-note--error">{error}</p>}
      </div>
    </Modal>
  );
}
