import { useState } from "react";
import { CheckCircle2, FileUp } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { Modal } from "@/components/Modal";
import { useStore } from "@/state/store";
import { errorMessage, type ImportSummary } from "@/lib/types";
import * as ipc from "@/lib/ipc";

export function RestoreBackupDialog() {
  const closeDialog = useStore((s) => s.closeDialog);
  const refreshAll = useStore((s) => s.refreshAll);
  const [path, setPath] = useState<string | null>(null);
  const [password, setPassword] = useState("");
  const [importAll, setImportAll] = useState(false);
  const [summary, setSummary] = useState<ImportSummary | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fileName = path ? path.split(/[\\/]/).pop() : null;

  const choose = async () => {
    const picked = await open({
      title: "Choose a ClovaKey backup",
      multiple: false,
      filters: [{ name: "ClovaKey backup", extensions: ["clovakey"] }],
    });
    if (typeof picked === "string") setPath(picked);
  };

  const restore = async () => {
    if (!path) return;
    setBusy(true);
    setError(null);
    try {
      const result = await ipc.backupRestore(path, password, importAll);
      setSummary(result);
      await refreshAll();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  if (summary) {
    return (
      <Modal title="Backup restored" onClose={closeDialog} size="sm">
        <div className="ck-done">
          <div className="ck-done__check">
            <CheckCircle2 size={44} />
          </div>
          <p className="ck-done__headline">
            Restored {summary.imported} account{summary.imported === 1 ? "" : "s"}.
          </p>
          {summary.skipped > 0 && (
            <p className="ck-note">{summary.skipped} duplicate(s) skipped.</p>
          )}
          {summary.groupsCreated > 0 && (
            <p className="ck-note">{summary.groupsCreated} group(s) created.</p>
          )}
          <button className="ck-btn ck-btn--primary" onClick={closeDialog}>
            Done
          </button>
        </div>
      </Modal>
    );
  }

  return (
    <Modal
      title="Restore from backup"
      subtitle="Open an encrypted .clovakey file and enter its passphrase."
      onClose={closeDialog}
      footer={
        <>
          <button className="ck-btn ck-btn--ghost" onClick={closeDialog} disabled={busy}>
            Cancel
          </button>
          <button
            className="ck-btn ck-btn--primary"
            onClick={restore}
            disabled={!path || !password || busy}
          >
            Restore accounts
          </button>
        </>
      }
    >
      <div className="ck-form">
        <button className="ck-filepick" onClick={choose}>
          <FileUp size={18} />
          <span>{fileName ?? "Choose backup file…"}</span>
        </button>
        <label className="ck-field">
          <span className="ck-field__label">Backup passphrase</span>
          <input
            className="ck-input"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && path && password && restore()}
          />
        </label>
        <label className="ck-checkline">
          <input
            type="checkbox"
            checked={importAll}
            onChange={(e) => setImportAll(e.target.checked)}
          />
          <span>Import accounts that already exist (don’t skip duplicates)</span>
        </label>
        {error && <p className="ck-note ck-note--error">{error}</p>}
      </div>
    </Modal>
  );
}
