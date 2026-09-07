import { KeyRound, Lock, ScanLine, Settings, Star, Tags } from "lucide-react";
import { useStore, type ViewId } from "@/state/store";

const NAV: { id: ViewId; label: string; icon: typeof KeyRound }[] = [
  { id: "auth", label: "Authenticator", icon: KeyRound },
  { id: "favorites", label: "Favorites", icon: Star },
  { id: "groups", label: "Groups", icon: Tags },
  { id: "import", label: "Import", icon: ScanLine },
  { id: "settings", label: "Settings", icon: Settings },
];

export function Sidebar() {
  const view = useStore((s) => s.view);
  const setView = useStore((s) => s.setView);
  const lock = useStore((s) => s.lock);
  const accounts = useStore((s) => s.accounts);
  const favCount = accounts.filter((a) => a.favorite).length;

  return (
    <nav className="ck-sidebar" aria-label="Primary">
      <div className="ck-sidebar__brand" data-tauri-drag-region>
        <img src="/clovakey.png" alt="" className="ck-sidebar__logo" />
        <span className="ck-sidebar__wordmark">ClovaKey</span>
      </div>

      <ul className="ck-nav">
        {NAV.map(({ id, label, icon: Icon }) => (
          <li key={id}>
            <button
              className={`ck-nav__item${view === id ? " is-active" : ""}`}
              onClick={() => setView(id)}
              aria-current={view === id ? "page" : undefined}
            >
              <Icon size={17} className="ck-nav__icon" />
              <span>{label}</span>
              {id === "auth" && accounts.length > 0 && (
                <span className="ck-nav__count">{accounts.length}</span>
              )}
              {id === "favorites" && favCount > 0 && (
                <span className="ck-nav__count">{favCount}</span>
              )}
            </button>
          </li>
        ))}
      </ul>

      <div className="ck-sidebar__foot">
        <button className="ck-nav__item ck-nav__lock" onClick={() => void lock()}>
          <Lock size={17} className="ck-nav__icon" />
          <span>Lock ClovaKey</span>
          <kbd className="ck-kbd">⌘L</kbd>
        </button>
      </div>
    </nav>
  );
}
