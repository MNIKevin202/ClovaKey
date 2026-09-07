import { useState } from "react";
import { Lock } from "lucide-react";
import { useStore } from "@/state/store";
import { errorMessage } from "@/lib/types";

export function LockScreen() {
  const protection = useStore((s) => s.status?.protection);
  const unlock = useStore((s) => s.unlock);
  const [passphrase, setPassphrase] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const needsPass = protection === "passphrase";

  const submit = async () => {
    setBusy(true);
    setError(null);
    try {
      await unlock(needsPass ? passphrase : null);
    } catch (e) {
      setError(errorMessage(e));
      setPassphrase("");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="ck-lock" data-tauri-drag-region>
      <div className="ck-lock__card">
        <div className="ck-lock__logo">
          <img src="/clovakey.png" alt="ClovaKey" width={76} height={76} />
          <span className="ck-lock__lockbadge">
            <Lock size={14} />
          </span>
        </div>
        <h1 className="ck-lock__title">ClovaKey is locked</h1>
        <p className="ck-lock__subtitle">
          {needsPass
            ? "Enter your passphrase to unlock your codes."
            : "Unlock to view and generate your authentication codes."}
        </p>

        <form
          className="ck-lock__form"
          onSubmit={(e) => {
            e.preventDefault();
            void submit();
          }}
        >
          {needsPass && (
            <input
              className="ck-input ck-input--lg"
              type="password"
              value={passphrase}
              onChange={(e) => setPassphrase(e.target.value)}
              placeholder="Passphrase"
              autoFocus
              aria-label="Passphrase"
            />
          )}
          <button
            className="ck-btn ck-btn--primary ck-btn--lg"
            type="submit"
            disabled={busy || (needsPass && !passphrase)}
          >
            Unlock
          </button>
        </form>

        {error && <p className="ck-note ck-note--error ck-lock__error">{error}</p>}
      </div>
    </div>
  );
}
