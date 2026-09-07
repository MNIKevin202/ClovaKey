import { useState } from "react";
import { AlertTriangle, Copy, Eye } from "lucide-react";
import { Modal } from "@/components/Modal";
import { useStore } from "@/state/store";
import { errorMessage, type RevealedSecret } from "@/lib/types";
import * as ipc from "@/lib/ipc";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

export function RevealSecretDialog({ accountId }: { accountId: string }) {
  const account = useStore((s) => s.accounts.find((a) => a.id === accountId));
  const protection = useStore((s) => s.status?.protection);
  const closeDialog = useStore((s) => s.closeDialog);
  const showToast = useStore((s) => s.showToast);

  const [passphrase, setPassphrase] = useState("");
  const [revealed, setRevealed] = useState<RevealedSecret | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const needsPass = protection === "passphrase";

  if (!account) {
    closeDialog();
    return null;
  }

  const reveal = async () => {
    setBusy(true);
    setError(null);
    try {
      const result = await ipc.accountReveal(account.id, needsPass ? passphrase : null);
      setRevealed(result);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const copy = async (value: string, label: string) => {
    try {
      await writeText(value);
      showToast(`${label} copied`, "success");
    } catch (e) {
      showToast(errorMessage(e), "danger");
    }
  };

  return (
    <Modal
      title="Reveal secret"
      subtitle={`${account.issuer ?? account.accountName}`}
      onClose={closeDialog}
      footer={
        revealed ? (
          <button className="ck-btn ck-btn--primary" onClick={closeDialog}>
            Done
          </button>
        ) : (
          <>
            <button className="ck-btn ck-btn--ghost" onClick={closeDialog} disabled={busy}>
              Cancel
            </button>
            <button
              className="ck-btn ck-btn--danger"
              onClick={reveal}
              disabled={busy || (needsPass && !passphrase)}
            >
              <Eye size={16} />
              Reveal
            </button>
          </>
        )
      }
    >
      {!revealed ? (
        <div className="ck-form">
          <div className="ck-callout ck-callout--danger">
            <AlertTriangle size={18} />
            <div>
              <strong>This is the account’s master secret.</strong> Anyone who has it can generate
              your codes. Only reveal it to move this account to another device, and keep it
              private.
            </div>
          </div>
          {needsPass && (
            <label className="ck-field">
              <span className="ck-field__label">Confirm your passphrase</span>
              <input
                className="ck-input"
                type="password"
                value={passphrase}
                onChange={(e) => setPassphrase(e.target.value)}
                autoFocus
                onKeyDown={(e) => e.key === "Enter" && passphrase && reveal()}
              />
            </label>
          )}
          {error && <p className="ck-note ck-note--error">{error}</p>}
        </div>
      ) : (
        <div className="ck-reveal">
          <div className="ck-reveal__qr" dangerouslySetInnerHTML={{ __html: revealed.qrSvg }} />
          <label className="ck-field">
            <span className="ck-field__label">Secret key</span>
            <div className="ck-copyrow">
              <code className="ck-code-mono">{revealed.secret}</code>
              <button
                className="ck-iconbtn"
                onClick={() => copy(revealed.secret, "Secret")}
                aria-label="Copy secret"
              >
                <Copy size={16} />
              </button>
            </div>
          </label>
          <label className="ck-field">
            <span className="ck-field__label">Setup link</span>
            <div className="ck-copyrow">
              <code className="ck-code-mono ck-code-mono--wrap">{revealed.uri}</code>
              <button
                className="ck-iconbtn"
                onClick={() => copy(revealed.uri, "Setup link")}
                aria-label="Copy setup link"
              >
                <Copy size={16} />
              </button>
            </div>
          </label>
        </div>
      )}
    </Modal>
  );
}
