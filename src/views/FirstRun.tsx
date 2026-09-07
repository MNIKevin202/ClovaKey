import { useState } from "react";
import { KeyRound, Plus, ShieldCheck } from "lucide-react";
import { useStore, type Dialog } from "@/state/store";
import { errorMessage } from "@/lib/types";

export function FirstRun() {
  const setup = useStore((s) => s.setup);
  const openDialog = useStore((s) => s.openDialog);
  const [usePassphrase, setUsePassphrase] = useState(false);
  const [passphrase, setPassphrase] = useState("");
  const [confirm, setConfirm] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const passphraseReady = !usePassphrase || (passphrase.length >= 4 && passphrase === confirm);

  const begin = async (next: Dialog) => {
    if (!passphraseReady) return;
    setBusy(true);
    setError(null);
    try {
      await setup(usePassphrase ? passphrase : null);
      openDialog(next);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="ck-firstrun" data-tauri-drag-region>
      <div className="ck-firstrun__inner">
        <img src="/clovakey.png" alt="" className="ck-firstrun__logo" width={104} height={104} />
        <h1 className="ck-firstrun__title">Welcome to ClovaKey</h1>
        <p className="ck-firstrun__tagline">
          Your authentication codes, encrypted and stored on your device. No account, no cloud —
          just your keys, ready when you need them.
        </p>

        <div className="ck-firstrun__actions">
          <button
            className="ck-btn ck-btn--primary ck-btn--lg"
            disabled={busy || !passphraseReady}
            onClick={() => begin({ kind: "add" })}
          >
            <Plus size={17} />
            Add an account
          </button>
          <button
            className="ck-btn ck-btn--ghost ck-btn--lg"
            disabled={busy || !passphraseReady}
            onClick={() => begin({ kind: "google" })}
          >
            <ShieldCheck size={17} />
            Import from Google Authenticator
          </button>
        </div>

        <div className="ck-firstrun__security">
          {!usePassphrase ? (
            <button className="ck-link" onClick={() => setUsePassphrase(true)}>
              <KeyRound size={14} />
              Protect with a master passphrase
            </button>
          ) : (
            <div className="ck-firstrun__passphrase">
              <p className="ck-note">
                Optional: a passphrase adds a hard lock and lets your backups move between machines.
                Keep it safe — it can’t be recovered.
              </p>
              <div className="ck-firstrun__pprow">
                <input
                  className="ck-input"
                  type="password"
                  placeholder="Passphrase"
                  value={passphrase}
                  onChange={(e) => setPassphrase(e.target.value)}
                />
                <input
                  className="ck-input"
                  type="password"
                  placeholder="Confirm"
                  value={confirm}
                  onChange={(e) => setConfirm(e.target.value)}
                />
              </div>
              <button className="ck-link" onClick={() => setUsePassphrase(false)}>
                Skip for now
              </button>
            </div>
          )}
        </div>

        {error && <p className="ck-note ck-note--error">{error}</p>}
        <p className="ck-firstrun__foot">Works completely offline · Nothing is ever uploaded</p>
      </div>
    </div>
  );
}
