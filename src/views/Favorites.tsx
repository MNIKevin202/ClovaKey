import { Star } from "lucide-react";
import { useStore } from "@/state/store";
import { SearchBar } from "@/components/SearchBar";
import { AccountList } from "@/components/AccountList";
import { EmptyState } from "@/components/EmptyState";

export function Favorites() {
  const accounts = useStore((s) => s.accounts);
  const search = useStore((s) => s.search);
  const q = search.toLowerCase().trim();

  let list = accounts.filter((a) => a.favorite);
  if (q) {
    list = list.filter((a) => `${a.issuer ?? ""} ${a.accountName}`.toLowerCase().includes(q));
  }

  const anyFavorites = accounts.some((a) => a.favorite);

  return (
    <div className="ck-view">
      <header className="ck-header" data-tauri-drag-region>
        <div className="ck-header__row">
          <h1 className="ck-header__title">Favorites</h1>
          <div className="ck-header__tools">{anyFavorites && <SearchBar />}</div>
        </div>
      </header>
      <div className="ck-scroll">
        {!anyFavorites ? (
          <EmptyState
            icon={<Star size={40} />}
            title="No favorites yet"
            message="Star the accounts you reach for most and they’ll appear here for quick access."
          />
        ) : (
          <AccountList accounts={list} />
        )}
      </div>
    </div>
  );
}
