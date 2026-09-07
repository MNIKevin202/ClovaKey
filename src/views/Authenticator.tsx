import { AlertTriangle, KeyRound, Plus, X } from "lucide-react";
import { useStore } from "@/state/store";
import { SearchBar } from "@/components/SearchBar";
import { AccountList } from "@/components/AccountList";
import { EmptyState } from "@/components/EmptyState";
import type { Account } from "@/lib/types";

function matches(a: Account, q: string, groupName: string | null): boolean {
  const hay = [a.issuer ?? "", a.accountName, groupName ?? ""].join(" ").toLowerCase();
  return hay.includes(q);
}

export function Authenticator() {
  const accounts = useStore((s) => s.accounts);
  const groups = useStore((s) => s.groups);
  const search = useStore((s) => s.search);
  const activeGroupId = useStore((s) => s.activeGroupId);
  const setActiveGroup = useStore((s) => s.setActiveGroup);
  const openDialog = useStore((s) => s.openDialog);
  const timeStatus = useStore((s) => s.timeStatus);

  const activeGroup = activeGroupId ? groups.find((g) => g.id === activeGroupId) : null;
  const groupName = (id: string | null) => groups.find((g) => g.id === id)?.name ?? null;

  const q = search.toLowerCase().trim();
  let list = accounts;
  if (activeGroupId) list = list.filter((a) => a.groupId === activeGroupId);
  if (q) list = list.filter((a) => matches(a, q, groupName(a.groupId)));

  const isEmpty = accounts.length === 0;

  return (
    <div className="ck-view">
      <header className="ck-header" data-tauri-drag-region>
        <div className="ck-header__row">
          <h1 className="ck-header__title">{activeGroup ? activeGroup.name : "Authenticator"}</h1>
          <div className="ck-header__tools">
            <SearchBar />
            <button className="ck-btn ck-btn--primary" onClick={() => openDialog({ kind: "add" })}>
              <Plus size={16} />
              Add account
            </button>
          </div>
        </div>
        {activeGroup && (
          <div className="ck-chiprow">
            <button className="ck-chip is-active" onClick={() => setActiveGroup(null)}>
              {activeGroup.name}
              <X size={13} />
            </button>
          </div>
        )}
      </header>

      <div className="ck-scroll">
        {timeStatus && !timeStatus.ok && timeStatus.reason && (
          <div className="ck-banner ck-banner--warning" role="alert">
            <AlertTriangle size={18} />
            <span>{timeStatus.reason}</span>
          </div>
        )}

        {isEmpty ? (
          <EmptyState
            icon={<img src="/clovakey.png" alt="" width={72} height={72} />}
            title="Your keys, when you need them"
            message="Add an authenticator or bring your existing accounts over from Google Authenticator. Everything stays encrypted on this device."
            actions={
              <>
                <button
                  className="ck-btn ck-btn--primary"
                  onClick={() => openDialog({ kind: "add" })}
                >
                  <Plus size={16} />
                  Add account
                </button>
                <button
                  className="ck-btn ck-btn--ghost"
                  onClick={() => openDialog({ kind: "google" })}
                >
                  Import from Google Authenticator
                </button>
              </>
            }
          />
        ) : list.length === 0 ? (
          <EmptyState
            icon={<KeyRound size={40} />}
            title="No matches"
            message={q ? `Nothing matches “${search}”.` : "This group has no accounts yet."}
          />
        ) : (
          <AccountList accounts={list} />
        )}
      </div>
    </div>
  );
}
