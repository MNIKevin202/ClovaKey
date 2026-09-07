import { useState } from "react";
import { ChevronDown } from "lucide-react";
import { Modal } from "@/components/Modal";
import { useStore } from "@/state/store";
import { errorMessage, type Algorithm, type OtpType } from "@/lib/types";
import * as ipc from "@/lib/ipc";

export function ManualEntryDialog() {
  const closeDialog = useStore((s) => s.closeDialog);
  const groups = useStore((s) => s.groups);
  const refreshAccounts = useStore((s) => s.refreshAccounts);
  const refreshCodes = useStore((s) => s.refreshCodes);
  const showToast = useStore((s) => s.showToast);

  const [issuer, setIssuer] = useState("");
  const [accountName, setAccountName] = useState("");
  const [secret, setSecret] = useState("");
  const [groupId, setGroupId] = useState<string>("");
  const [type, setType] = useState<OtpType>("totp");
  const [algorithm, setAlgorithm] = useState<Algorithm>("SHA1");
  const [digits, setDigits] = useState(6);
  const [period, setPeriod] = useState(30);
  const [counter, setCounter] = useState(0);
  const [advanced, setAdvanced] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const canSubmit = secret.trim().length > 0 && (accountName.trim() || issuer.trim());

  const submit = async () => {
    if (!canSubmit) return;
    setBusy(true);
    setError(null);
    try {
      await ipc.accountAddManual({
        issuer: issuer.trim() || null,
        accountName: accountName.trim(),
        secret: secret.trim(),
        type,
        algorithm,
        digits,
        period,
        counter,
        groupId: groupId || null,
        favorite: false,
      });
      await Promise.all([refreshAccounts(), refreshCodes()]);
      showToast("Account added", "success");
      closeDialog();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title="Enter a setup key"
      subtitle="Only the secret key is required — the rest have sensible defaults."
      onClose={closeDialog}
      footer={
        <>
          <button className="ck-btn ck-btn--ghost" onClick={closeDialog} disabled={busy}>
            Cancel
          </button>
          <button className="ck-btn ck-btn--primary" onClick={submit} disabled={!canSubmit || busy}>
            Add account
          </button>
        </>
      }
    >
      <div className="ck-form">
        <label className="ck-field">
          <span className="ck-field__label">Service / issuer</span>
          <input
            className="ck-input"
            value={issuer}
            onChange={(e) => setIssuer(e.target.value)}
            placeholder="e.g. GitHub"
            autoFocus
          />
        </label>

        <label className="ck-field">
          <span className="ck-field__label">Account name</span>
          <input
            className="ck-input"
            value={accountName}
            onChange={(e) => setAccountName(e.target.value)}
            placeholder="e.g. you@example.com"
          />
        </label>

        <label className="ck-field">
          <span className="ck-field__label">Secret key</span>
          <input
            className="ck-input ck-input--mono"
            value={secret}
            onChange={(e) => setSecret(e.target.value)}
            placeholder="Base32 secret"
            autoComplete="off"
            spellCheck={false}
          />
        </label>

        {groups.length > 0 && (
          <label className="ck-field">
            <span className="ck-field__label">Group</span>
            <div className="ck-select">
              <select value={groupId} onChange={(e) => setGroupId(e.target.value)}>
                <option value="">No group</option>
                {groups.map((g) => (
                  <option key={g.id} value={g.id}>
                    {g.name}
                  </option>
                ))}
              </select>
              <ChevronDown size={15} />
            </div>
          </label>
        )}

        <button
          type="button"
          className={`ck-disclosure${advanced ? " is-open" : ""}`}
          onClick={() => setAdvanced((v) => !v)}
          aria-expanded={advanced}
        >
          <ChevronDown size={15} />
          Advanced options
        </button>

        {advanced && (
          <div className="ck-form__grid">
            <label className="ck-field">
              <span className="ck-field__label">Type</span>
              <div className="ck-select">
                <select value={type} onChange={(e) => setType(e.target.value as OtpType)}>
                  <option value="totp">Time-based (TOTP)</option>
                  <option value="hotp">Counter-based (HOTP)</option>
                </select>
                <ChevronDown size={15} />
              </div>
            </label>

            <label className="ck-field">
              <span className="ck-field__label">Algorithm</span>
              <div className="ck-select">
                <select
                  value={algorithm}
                  onChange={(e) => setAlgorithm(e.target.value as Algorithm)}
                >
                  <option value="SHA1">SHA-1</option>
                  <option value="SHA256">SHA-256</option>
                  <option value="SHA512">SHA-512</option>
                </select>
                <ChevronDown size={15} />
              </div>
            </label>

            <label className="ck-field">
              <span className="ck-field__label">Digits</span>
              <div className="ck-select">
                <select value={digits} onChange={(e) => setDigits(Number(e.target.value))}>
                  <option value={6}>6</option>
                  <option value={7}>7</option>
                  <option value={8}>8</option>
                </select>
                <ChevronDown size={15} />
              </div>
            </label>

            {type === "totp" ? (
              <label className="ck-field">
                <span className="ck-field__label">Period (seconds)</span>
                <input
                  className="ck-input"
                  type="number"
                  min={1}
                  max={3600}
                  value={period}
                  onChange={(e) => setPeriod(Number(e.target.value))}
                />
              </label>
            ) : (
              <label className="ck-field">
                <span className="ck-field__label">Counter</span>
                <input
                  className="ck-input"
                  type="number"
                  min={0}
                  value={counter}
                  onChange={(e) => setCounter(Number(e.target.value))}
                />
              </label>
            )}
          </div>
        )}

        {error && <p className="ck-note ck-note--error">{error}</p>}
      </div>
    </Modal>
  );
}
