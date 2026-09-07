import { useState } from "react";
import { Check, ChevronRight, MoreHorizontal, Pencil, Plus, Tags, Trash2, X } from "lucide-react";
import { useStore } from "@/state/store";
import { EmptyState } from "@/components/EmptyState";
import { Menu } from "@/components/Menu";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { pluralize } from "@/lib/format";
import { errorMessage } from "@/lib/types";
import * as ipc from "@/lib/ipc";

export function GroupsView() {
  const groups = useStore((s) => s.groups);
  const accounts = useStore((s) => s.accounts);
  const refreshGroups = useStore((s) => s.refreshGroups);
  const refreshAccounts = useStore((s) => s.refreshAccounts);
  const setActiveGroup = useStore((s) => s.setActiveGroup);
  const setView = useStore((s) => s.setView);
  const showToast = useStore((s) => s.showToast);

  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [deleteId, setDeleteId] = useState<string | null>(null);

  const countFor = (id: string) => accounts.filter((a) => a.groupId === id).length;

  const create = async () => {
    const name = newName.trim();
    if (!name) return;
    try {
      await ipc.groupCreate(name);
      setNewName("");
      setCreating(false);
      await refreshGroups();
    } catch (e) {
      showToast(errorMessage(e), "danger");
    }
  };

  const rename = async (id: string) => {
    const name = editName.trim();
    if (!name) return;
    try {
      await ipc.groupRename(id, name);
      setEditingId(null);
      await refreshGroups();
    } catch (e) {
      showToast(errorMessage(e), "danger");
    }
  };

  const open = (id: string) => {
    setActiveGroup(id);
    setView("auth");
  };

  const deleting = deleteId ? groups.find((g) => g.id === deleteId) : null;

  return (
    <div className="ck-view">
      <header className="ck-header" data-tauri-drag-region>
        <div className="ck-header__row">
          <h1 className="ck-header__title">Groups</h1>
          <div className="ck-header__tools">
            <button className="ck-btn ck-btn--primary" onClick={() => setCreating(true)}>
              <Plus size={16} />
              New group
            </button>
          </div>
        </div>
      </header>

      <div className="ck-scroll">
        {creating && (
          <div className="ck-grouprow ck-grouprow--edit">
            <Tags size={18} className="ck-grouprow__icon" />
            <input
              className="ck-input ck-input--inline"
              autoFocus
              value={newName}
              placeholder="Group name"
              onChange={(e) => setNewName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void create();
                if (e.key === "Escape") {
                  setCreating(false);
                  setNewName("");
                }
              }}
            />
            <button className="ck-iconbtn" onClick={() => void create()} aria-label="Save group">
              <Check size={16} />
            </button>
            <button
              className="ck-iconbtn"
              onClick={() => {
                setCreating(false);
                setNewName("");
              }}
              aria-label="Cancel"
            >
              <X size={16} />
            </button>
          </div>
        )}

        {groups.length === 0 && !creating ? (
          <EmptyState
            icon={<Tags size={40} />}
            title="No groups yet"
            message="Groups are optional. Create groups like Personal, Work, or Finance to organize your accounts."
            actions={
              <button className="ck-btn ck-btn--primary" onClick={() => setCreating(true)}>
                <Plus size={16} />
                New group
              </button>
            }
          />
        ) : (
          <div className="ck-grouplist">
            {groups.map((g) =>
              editingId === g.id ? (
                <div key={g.id} className="ck-grouprow ck-grouprow--edit">
                  <Tags size={18} className="ck-grouprow__icon" />
                  <input
                    className="ck-input ck-input--inline"
                    autoFocus
                    value={editName}
                    onChange={(e) => setEditName(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") void rename(g.id);
                      if (e.key === "Escape") setEditingId(null);
                    }}
                  />
                  <button
                    className="ck-iconbtn"
                    onClick={() => void rename(g.id)}
                    aria-label="Save"
                  >
                    <Check size={16} />
                  </button>
                  <button
                    className="ck-iconbtn"
                    onClick={() => setEditingId(null)}
                    aria-label="Cancel"
                  >
                    <X size={16} />
                  </button>
                </div>
              ) : (
                <div key={g.id} className="ck-grouprow">
                  <button className="ck-grouprow__main" onClick={() => open(g.id)}>
                    <Tags size={18} className="ck-grouprow__icon" />
                    <span className="ck-grouprow__name">{g.name}</span>
                    <span className="ck-grouprow__count">
                      {countFor(g.id)} {pluralize(countFor(g.id), "account")}
                    </span>
                    <ChevronRight size={16} className="ck-grouprow__chev" />
                  </button>
                  <Menu
                    align="right"
                    trigger={(p) => (
                      <button className="ck-iconbtn" aria-label={`Options for ${g.name}`} {...p}>
                        <MoreHorizontal size={18} />
                      </button>
                    )}
                    items={[
                      {
                        label: "Rename",
                        icon: <Pencil size={15} />,
                        onClick: () => {
                          setEditingId(g.id);
                          setEditName(g.name);
                        },
                      },
                      { separator: true },
                      {
                        label: "Delete group",
                        icon: <Trash2 size={15} />,
                        danger: true,
                        onClick: () => setDeleteId(g.id),
                      },
                    ]}
                  />
                </div>
              ),
            )}
          </div>
        )}
      </div>

      {deleting && (
        <ConfirmDialog
          title={`Delete “${deleting.name}”?`}
          message="The group will be removed. Accounts in this group are kept and simply become ungrouped."
          confirmLabel="Delete group"
          danger
          onClose={() => setDeleteId(null)}
          onConfirm={async () => {
            try {
              await ipc.groupDelete(deleting.id);
              await Promise.all([refreshGroups(), refreshAccounts()]);
            } catch (e) {
              showToast(errorMessage(e), "danger");
            }
          }}
        />
      )}
    </div>
  );
}
