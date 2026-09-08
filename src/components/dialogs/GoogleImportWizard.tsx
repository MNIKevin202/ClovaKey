import { useEffect, useRef, useState } from "react";
import { CheckCircle2, ChevronDown, Image as ImageIcon, Link2, Pencil, QrCode } from "lucide-react";
import { Modal } from "@/components/Modal";
import { ServiceIcon } from "@/components/ServiceIcon";
import { useStore, type GoogleSeed } from "@/state/store";
import { fileToBase64 } from "@/lib/image";
import {
  errorMessage,
  type BatchProgress,
  type ImportSelection,
  type ImportSummary,
  type PreviewItem,
} from "@/lib/types";
import * as ipc from "@/lib/ipc";

type Stage = "add" | "preview" | "done";
type Edit = { issuer: string; accountName: string };

export function GoogleImportWizard({ seed }: { seed?: GoogleSeed }) {
  const closeDialog = useStore((s) => s.closeDialog);
  const groups = useStore((s) => s.groups);
  const refreshAll = useStore((s) => s.refreshAll);

  const sessionRef = useRef<string | null>(null);
  const seededRef = useRef(false);
  const fileRef = useRef<HTMLInputElement>(null);

  const [stage, setStage] = useState<Stage>("add");
  // Mirrored into a ref purely so the unmount cleanup below can read the current
  // stage without taking a dependency on it.
  const stageRef = useRef(stage);
  stageRef.current = stage;
  const [progress, setProgress] = useState<BatchProgress | null>(null);
  const [items, setItems] = useState<PreviewItem[]>([]);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [edits, setEdits] = useState<Record<number, Edit>>({});
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [groupId, setGroupId] = useState("");
  const [pasteText, setPasteText] = useState("");
  const [showPaste, setShowPaste] = useState(false);
  const [summary, setSummary] = useState<ImportSummary | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const ensureSession = async (): Promise<string> => {
    if (!sessionRef.current) sessionRef.current = await ipc.googleImportBegin();
    return sessionRef.current;
  };

  const loadPreview = async (sid: string) => {
    const preview = await ipc.googleImportPreview(sid);
    setItems(preview);
    // Default: select everything that isn't a duplicate.
    setSelected(new Set(preview.filter((p) => !p.duplicate).map((p) => p.index)));
    setStage("preview");
  };

  const afterProgress = async (sid: string, p: BatchProgress) => {
    setProgress(p);
    if (p.complete) await loadPreview(sid);
  };

  const addImage = async (b64: string) => {
    setError(null);
    setBusy(true);
    try {
      const sid = await ensureSession();
      const p = await ipc.googleImportAddImage(sid, b64);
      await afterProgress(sid, p);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const addText = async (text: string) => {
    setError(null);
    setBusy(true);
    try {
      const sid = await ensureSession();
      const p = await ipc.googleImportAddText(sid, text);
      await afterProgress(sid, p);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  // Consume a seed handed over from a scan/paste that detected a migration QR.
  useEffect(() => {
    if (seededRef.current || !seed) return;
    seededRef.current = true;
    if (seed.kind === "image") void addImage(seed.data);
    else void addText(seed.data);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Cancel the session if the user closes mid-flow (zeroizes staged secrets).
  //
  // This must run on unmount only. Depending on `stage` made React run the cleanup
  // on every stage change, so advancing from "add" to "review" — the moment the
  // accounts are ready to import — cancelled the very session the import needs, and
  // committing then failed every time. The stage is read through a ref instead.
  useEffect(() => {
    return () => {
      if (sessionRef.current && stageRef.current !== "done") {
        void ipc.googleImportCancel(sessionRef.current);
      }
    };
  }, []);

  const toggle = (index: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(index)) next.delete(index);
      else next.add(index);
      return next;
    });
  };

  const commit = async () => {
    const sid = sessionRef.current;
    if (!sid) return;
    setBusy(true);
    setError(null);
    try {
      const selections: ImportSelection[] = [...selected].map((index) => ({
        index,
        issuer: edits[index]?.issuer ?? null,
        accountName: edits[index]?.accountName ?? null,
        groupId: groupId || null,
        favorite: false,
      }));
      const result = await ipc.googleImportCommit(sid, selections);
      setSummary(result);
      setStage("done");
      await refreshAll();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const duplicateCount = items.filter((i) => i.duplicate).length;

  // ── Stage: add QR codes ───────────────────────────────────────────────
  if (stage === "add") {
    return (
      <Modal
        title="Import from Google Authenticator"
        subtitle="In Google Authenticator, tap ⋯ → Transfer accounts → Export, then bring the QR code here. It’s decoded entirely on your device."
        onClose={closeDialog}
      >
        <div className="ck-gimport">
          {progress && progress.total > 1 && (
            <div className="ck-batch">
              <div className="ck-batch__head">
                <QrCode size={16} />
                <span>
                  QR codes scanned <strong>{progress.received}</strong> of{" "}
                  <strong>{progress.total}</strong>
                </span>
              </div>
              <div className="ck-batch__track">
                {Array.from({ length: progress.total }).map((_, i) => (
                  <span
                    key={i}
                    className={`ck-batch__pip${i < progress.received ? " is-on" : ""}`}
                  />
                ))}
              </div>
              <p className="ck-note">
                This export is split across multiple QR codes. Add the remaining codes to continue.
              </p>
            </div>
          )}

          {busy && <p className="ck-note">Decoding…</p>}
          {error && <p className="ck-note ck-note--error">{error}</p>}

          <div className="ck-gimport__methods">
            <button
              className="ck-btn ck-btn--primary"
              onClick={() => fileRef.current?.click()}
              disabled={busy}
            >
              <ImageIcon size={16} />
              {progress ? "Add next QR image" : "Import QR image"}
            </button>
            <button
              className="ck-btn ck-btn--ghost"
              onClick={() => setShowPaste((v) => !v)}
              disabled={busy}
            >
              <Link2 size={16} />
              Paste export link
            </button>
          </div>

          {showPaste && (
            <div className="ck-form">
              <textarea
                className="ck-input ck-input--mono ck-textarea"
                rows={3}
                placeholder="otpauth-migration://offline?data=..."
                value={pasteText}
                onChange={(e) => setPasteText(e.target.value)}
                spellCheck={false}
              />
              <button
                className="ck-btn ck-btn--primary"
                disabled={!pasteText.trim() || busy}
                onClick={() => {
                  void addText(pasteText.trim());
                  setPasteText("");
                }}
              >
                Add this code
              </button>
            </div>
          )}
        </div>

        <input
          ref={fileRef}
          type="file"
          accept="image/*"
          hidden
          onChange={async (e) => {
            const file = e.target.files?.[0];
            if (file) await addImage(await fileToBase64(file));
            e.target.value = "";
          }}
        />
      </Modal>
    );
  }

  // ── Stage: preview & select ───────────────────────────────────────────
  if (stage === "preview") {
    return (
      <Modal
        title={`Review ${items.length} account${items.length === 1 ? "" : "s"}`}
        subtitle={
          duplicateCount > 0
            ? `${duplicateCount} look like accounts you already have and are unchecked.`
            : "Choose which accounts to import. You can rename them first."
        }
        onClose={closeDialog}
        size="lg"
        footer={
          <>
            <button className="ck-btn ck-btn--ghost" onClick={closeDialog} disabled={busy}>
              Cancel
            </button>
            <button
              className="ck-btn ck-btn--primary"
              onClick={commit}
              disabled={selected.size === 0 || busy}
            >
              Import {selected.size} selected
            </button>
          </>
        }
      >
        <div className="ck-preview">
          <div className="ck-preview__bar">
            <div className="ck-preview__selectors">
              <button
                className="ck-link"
                onClick={() => setSelected(new Set(items.map((i) => i.index)))}
              >
                Select all
              </button>
              <span className="ck-dot" />
              <button className="ck-link" onClick={() => setSelected(new Set())}>
                Select none
              </button>
            </div>
            {groups.length > 0 && (
              <label className="ck-preview__group">
                <span>Add to</span>
                <div className="ck-select ck-select--sm">
                  <select value={groupId} onChange={(e) => setGroupId(e.target.value)}>
                    <option value="">No group</option>
                    {groups.map((g) => (
                      <option key={g.id} value={g.id}>
                        {g.name}
                      </option>
                    ))}
                  </select>
                  <ChevronDown size={14} />
                </div>
              </label>
            )}
          </div>

          <ul className="ck-preview__list">
            {items.map((item) => {
              const edit = edits[item.index];
              const issuer = edit?.issuer ?? item.issuer ?? "";
              const name = edit?.accountName ?? item.accountName;
              const isEditing = editingIndex === item.index;
              return (
                <li
                  key={item.index}
                  className={`ck-prow${selected.has(item.index) ? " is-selected" : ""}`}
                >
                  <input
                    type="checkbox"
                    className="ck-check"
                    checked={selected.has(item.index)}
                    onChange={() => toggle(item.index)}
                    aria-label={`Import ${issuer || name}`}
                  />
                  <ServiceIcon issuer={issuer || null} accountName={name} size={34} />
                  {isEditing ? (
                    <div className="ck-prow__edit">
                      <input
                        className="ck-input ck-input--sm"
                        value={issuer}
                        placeholder="Issuer"
                        onChange={(e) =>
                          setEdits((m) => ({
                            ...m,
                            [item.index]: { issuer: e.target.value, accountName: name },
                          }))
                        }
                      />
                      <input
                        className="ck-input ck-input--sm"
                        value={name}
                        placeholder="Account"
                        onChange={(e) =>
                          setEdits((m) => ({
                            ...m,
                            [item.index]: { issuer, accountName: e.target.value },
                          }))
                        }
                        onKeyDown={(e) => e.key === "Enter" && setEditingIndex(null)}
                      />
                    </div>
                  ) : (
                    <div className="ck-prow__labels">
                      <span className="ck-prow__primary">{issuer || name}</span>
                      {issuer && <span className="ck-prow__secondary">{name}</span>}
                    </div>
                  )}
                  <div className="ck-prow__meta">
                    {item.duplicate && <span className="ck-badge ck-badge--warn">Duplicate</span>}
                    <span className="ck-badge">{item.type.toUpperCase()}</span>
                    <button
                      className="ck-iconbtn"
                      aria-label={`Edit ${issuer || name}`}
                      onClick={() => setEditingIndex(isEditing ? null : item.index)}
                    >
                      <Pencil size={14} />
                    </button>
                  </div>
                </li>
              );
            })}
          </ul>

          {error && <p className="ck-note ck-note--error">{error}</p>}
        </div>
      </Modal>
    );
  }

  // ── Stage: done ───────────────────────────────────────────────────────
  return (
    <Modal title="Import complete" onClose={closeDialog} size="sm">
      <div className="ck-done">
        <div className="ck-done__check">
          <CheckCircle2 size={44} />
        </div>
        <p className="ck-done__headline">
          Imported {summary?.imported ?? 0} account{summary?.imported === 1 ? "" : "s"}.
        </p>
        {summary && summary.skipped > 0 && (
          <p className="ck-note">{summary.skipped} duplicate(s) were skipped.</p>
        )}
        {summary && summary.groupsCreated > 0 && (
          <p className="ck-note">{summary.groupsCreated} group(s) created.</p>
        )}
        <button className="ck-btn ck-btn--primary" onClick={closeDialog}>
          Done
        </button>
      </div>
    </Modal>
  );
}
