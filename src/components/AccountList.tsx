import { useStore } from "@/state/store";
import type { Account } from "@/lib/types";
import { AccountCard } from "./AccountCard";

interface Props {
  accounts: Account[];
}

/** Renders a list of account cards, wired to live codes + the tick clock. */
export function AccountList({ accounts }: Props) {
  const codes = useStore((s) => s.codes);
  const now = useStore((s) => s.now);

  return (
    <div className="ck-list" role="list">
      {accounts.map((account) => (
        <div role="listitem" key={account.id}>
          <AccountCard account={account} code={codes[account.id]} now={now} />
        </div>
      ))}
    </div>
  );
}
