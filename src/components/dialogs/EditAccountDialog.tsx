import { useState } from "react";
import { ChevronDown, Star } from "lucide-react";
import { Modal } from "@/components/Modal";
import { useStore } from "@/state/store";
import { errorMessage } from "@/lib/types";
import * as ipc from "@/lib/ipc";

export function EditAccountDialog({ accountId }: { accountId: string }) {
  const account = useStore((s) => s.accounts.find((a) => a.id === accountId));
  const groups = useStore((s) => s.groups);
  const closeDialog = useStore((s) => s.closeDialog);
  const refreshAccounts = useStore((s) => s.refreshAccounts);
  const showToast = useStore((s) => s.showToast);

  const [issuer, setIssuer] = useState(account?.issuer ?? "");
  const [accountName, setAccountName] = useState(account?.accountName ?? "");
  const [groupId, setGroupId] = useState(account?.groupId ?? "");
  const [favorite, setFavorite] = useState(account?.favorite ?? false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!account) {
    closeDialog();
    return null;
  }

  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      await ipc.accountUpdate(account.id, {
        issuer: issuer.trim() || null,
        accountName: accountName.trim() || account.accountName,
        groupId: groupId || null,
        favorite,
      });
      await refreshAccounts();
      showToast("Changes saved", "success");
      closeDialog();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const tech = [
    ["Type", account.type.toUpperCase()],
    ["Algorithm", account.algorithm],
    ["Digits", String(account.digits)],
    account.type === "totp"
      ? ["Period", `${account.period}s`]
      : ["Counter", String(account.counter)],
  ];

  return (
    <Modal
      title="Edit account"
      onClose={closeDialog}
      footer={
        <>
          <button className="ck-btn ck-btn--ghost" onClick={closeDialog} disabled={busy}>
            Cancel
          </button>
          <button className="ck-btn ck-btn--primary" onClick={save} disabled={busy}>
            Save changes
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
            autoFocus
          />
        </label>
        <label className="ck-field">
          <span className="ck-field__label">Account name</span>
          <input
            className="ck-input"
            value={accountName}
            onChange={(e) => setAccountName(e.target.value)}
          />
        </label>
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

        <button
          type="button"
          className={`ck-toggle${favorite ? " is-on" : ""}`}
          onClick={() => setFavorite((v) => !v)}
          aria-pressed={favorite}
        >
          <Star size={16} fill={favorite ? "currentColor" : "none"} />
          {favorite ? "Favorited" : "Add to favorites"}
        </button>

        <div className="ck-techbox">
          <span className="ck-techbox__title">Technical configuration</span>
          <dl className="ck-techlist">
            {tech.map(([k, v]) => (
              <div key={k} className="ck-techlist__row">
                <dt>{k}</dt>
                <dd>{v}</dd>
              </div>
            ))}
          </dl>
        </div>

        {error && <p className="ck-note ck-note--error">{error}</p>}
      </div>
    </Modal>
  );
}
