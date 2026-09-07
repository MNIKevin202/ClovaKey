import { Search, X } from "lucide-react";
import { useStore } from "@/state/store";

export const SEARCH_INPUT_ID = "ck-search-input";

export function SearchBar({ placeholder = "Search" }: { placeholder?: string }) {
  const search = useStore((s) => s.search);
  const setSearch = useStore((s) => s.setSearch);

  return (
    <div className="ck-search">
      <Search size={16} className="ck-search__icon" aria-hidden="true" />
      <input
        id={SEARCH_INPUT_ID}
        className="ck-search__input"
        type="text"
        value={search}
        placeholder={placeholder}
        onChange={(e) => setSearch(e.target.value)}
        aria-label="Search accounts"
        autoComplete="off"
        spellCheck={false}
      />
      {search && (
        <button
          className="ck-search__clear"
          onClick={() => setSearch("")}
          aria-label="Clear search"
        >
          <X size={14} />
        </button>
      )}
    </div>
  );
}
