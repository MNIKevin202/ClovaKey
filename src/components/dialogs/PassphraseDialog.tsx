import { useState } from "react";
import { Modal } from "@/components/Modal";
import { useStore } from "@/state/store";
import { errorMessage } from "@/lib/types";
import * as ipc from "@/lib/ipc";

export type PassphraseMode = "set" | "change" | "remove";

interface Props {
  mode: PassphraseMode;
  onClose: () => void;
}

const COPY: Record<PassphraseMode, { title: string; subtitle: string; cta: string }> = {
  set: {
    title: "Set a passphrase",
    subtitle:
      "A passphrase adds a hard lock: ClovaKey can’t generate codes until you enter it. Choose something you won’t forget — it can’t be recovered.",
    cta: "Set passphrase",
  },
  change: {
    title: "Change passphrase",
    subtitle: "Enter your current passphrase, then choose a new one.",
    cta: "Change passphrase",
  },
  remove: {
    title: "Remove passphrase",
    subtitle:
      "ClovaKey will fall back to protecting your vault with the system keychain. The app will no longer ask for a passphrase to unlock.",
    cta: "Remove passphrase",
  },
};

export function PassphraseDialog({ mode, onClose }: Props) {
  const refreshStatus = useStore((s) => s.refreshStatus);
  const showToast = useStore((s) => s.showToast);
  const [oldPass, setOldPass] = useState("");
  const [newPass, setNewPass] = useState("");
  const [confirm, setConfirm] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const copy = COPY[mode];
  const needsOld = mode === "change" || mode === "remove";
  const needsNew = mode === "set" || mode === "change";

  const valid =
    (!needsOld || oldPass.length > 0) && (!needsNew || (newPass.length > 0 && newPass === confirm));

  const submit = async () => {
    if (!valid) return;
    setBusy(true);
    setError(null);
    try {
      if (mode === "set") await ipc.vaultSetPassphrase(newPass);
      else if (mode === "change") await ipc.vaultChangePassphrase(oldPass, newPass);
      else await ipc.vaultRemovePassphrase(oldPass);
      await refreshStatus();
      showToast(mode === "remove" ? "Passphrase removed" : "Passphrase updated", "success");
      onClose();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title={copy.title}
      subtitle={copy.subtitle}
      onClose={onClose}
      size="sm"
      footer={
        <>
          <button className="ck-btn ck-btn--ghost" onClick={onClose} disabled={busy}>
            Cancel
          </button>
          <button
            className={`ck-btn ${mode === "remove" ? "ck-btn--danger" : "ck-btn--primary"}`}
            onClick={submit}
            disabled={!valid || busy}
          >
            {copy.cta}
          </button>
        </>
      }
    >
      <div className="ck-form">
        {needsOld && (
          <label className="ck-field">
            <span className="ck-field__label">Current passphrase</span>
            <input
              className="ck-input"
              type="password"
              value={oldPass}
              onChange={(e) => setOldPass(e.target.value)}
              autoFocus
            />
          </label>
        )}
        {needsNew && (
          <>
            <label className="ck-field">
              <span className="ck-field__label">New passphrase</span>
              <input
                className="ck-input"
                type="password"
                value={newPass}
                onChange={(e) => setNewPass(e.target.value)}
                autoFocus={!needsOld}
              />
            </label>
            <label className="ck-field">
              <span className="ck-field__label">Confirm new passphrase</span>
              <input
                className="ck-input"
                type="password"
                value={confirm}
                onChange={(e) => setConfirm(e.target.value)}
              />
              {confirm.length > 0 && confirm !== newPass && (
                <span className="ck-field__hint ck-field__hint--warn">
                  Passphrases don’t match.
                </span>
              )}
            </label>
          </>
        )}
        {error && <p className="ck-note ck-note--error">{error}</p>}
      </div>
    </Modal>
  );
}
