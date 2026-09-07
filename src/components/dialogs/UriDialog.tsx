import { useState } from "react";
import { Modal } from "@/components/Modal";
import { useStore } from "@/state/store";
import { errorMessage } from "@/lib/types";
import * as ipc from "@/lib/ipc";

export function UriDialog() {
  const closeDialog = useStore((s) => s.closeDialog);
  const openDialog = useStore((s) => s.openDialog);
  const refreshAccounts = useStore((s) => s.refreshAccounts);
  const refreshCodes = useStore((s) => s.refreshCodes);
  const showToast = useStore((s) => s.showToast);
  const [uri, setUri] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async () => {
    const text = uri.trim();
    if (!text) return;
    setBusy(true);
    setError(null);
    try {
      const outcome = await ipc.scanQrText(text);
      if (outcome.kind === "otpauth" && outcome.account) {
        await Promise.all([refreshAccounts(), refreshCodes()]);
        showToast("Account added", "success");
        closeDialog();
      } else if (outcome.kind === "migration") {
        openDialog({ kind: "google", seed: { kind: "text", data: text } });
      } else {
        setError("That doesn’t look like an otpauth:// setup link.");
      }
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title="Paste a setup link"
      subtitle="Paste an otpauth:// link. It’s parsed and stored locally."
      onClose={closeDialog}
      footer={
        <>
          <button className="ck-btn ck-btn--ghost" onClick={closeDialog} disabled={busy}>
            Cancel
          </button>
          <button
            className="ck-btn ck-btn--primary"
            onClick={submit}
            disabled={!uri.trim() || busy}
          >
            Add account
          </button>
        </>
      }
    >
      <div className="ck-form">
        <label className="ck-field">
          <span className="ck-field__label">Setup link</span>
          <textarea
            className="ck-input ck-input--mono ck-textarea"
            value={uri}
            onChange={(e) => setUri(e.target.value)}
            placeholder="otpauth://totp/Issuer:account?secret=..."
            rows={4}
            autoFocus
            spellCheck={false}
          />
        </label>
        {error && <p className="ck-note ck-note--error">{error}</p>}
      </div>
    </Modal>
  );
}
