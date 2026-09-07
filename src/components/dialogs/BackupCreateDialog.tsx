import { useState } from "react";
import { AlertTriangle, ShieldCheck } from "lucide-react";
import { save } from "@tauri-apps/plugin-dialog";
import { Modal } from "@/components/Modal";
import { useStore } from "@/state/store";
import { errorMessage } from "@/lib/types";
import * as ipc from "@/lib/ipc";

export function BackupCreateDialog() {
  const closeDialog = useStore((s) => s.closeDialog);
  const showToast = useStore((s) => s.showToast);
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const tooShort = password.length > 0 && password.length < 8;
  const mismatch = confirm.length > 0 && confirm !== password;
  const canCreate = password.length >= 8 && confirm === password && !busy;

  const create = async () => {
    setBusy(true);
    setError(null);
    try {
      const date = new Date().toISOString().slice(0, 10);
      const path = await save({
        title: "Save ClovaKey backup",
        defaultPath: `ClovaKey Backup ${date}.clovakey`,
        filters: [{ name: "ClovaKey backup", extensions: ["clovakey"] }],
      });
      if (!path) {
        setBusy(false);
        return;
      }
      await ipc.backupCreate(path, password);
      showToast("Backup saved", "success");
      closeDialog();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title="Create encrypted backup"
      subtitle="Your accounts are encrypted with a passphrase you choose, so the backup is portable to another computer."
      onClose={closeDialog}
      footer={
        <>
          <button className="ck-btn ck-btn--ghost" onClick={closeDialog} disabled={busy}>
            Cancel
          </button>
          <button className="ck-btn ck-btn--primary" onClick={create} disabled={!canCreate}>
            <ShieldCheck size={16} />
            Choose location & save
          </button>
        </>
      }
    >
      <div className="ck-form">
        <div className="ck-callout">
          <AlertTriangle size={18} />
          <div>
            This passphrase is the only way to open the backup. If you forget it, the backup can’t
            be recovered — ClovaKey never stores it.
          </div>
        </div>
        <label className="ck-field">
          <span className="ck-field__label">Backup passphrase</span>
          <input
            className="ck-input"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoFocus
          />
          {tooShort && (
            <span className="ck-field__hint ck-field__hint--warn">Use at least 8 characters.</span>
          )}
        </label>
        <label className="ck-field">
          <span className="ck-field__label">Confirm passphrase</span>
          <input
            className="ck-input"
            type="password"
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
          />
          {mismatch && (
            <span className="ck-field__hint ck-field__hint--warn">Passphrases don’t match.</span>
          )}
        </label>
        {error && <p className="ck-note ck-note--error">{error}</p>}
      </div>
    </Modal>
  );
}
