import { memo, useState } from "react";
import { Copy, Eye, MoreHorizontal, Pencil, RefreshCw, Star, Trash2 } from "lucide-react";
import type { Account, GeneratedCode } from "@/lib/types";
import { groupCode, primaryLabel, secondaryLabel } from "@/lib/format";
import { ServiceIcon } from "./ServiceIcon";
import { CountdownRing } from "./CountdownRing";
import { Menu } from "./Menu";
import { useStore } from "@/state/store";
import * as ipc from "@/lib/ipc";
import { errorMessage } from "@/lib/types";

interface Props {
  account: Account;
  code: GeneratedCode | undefined;
  now: number;
}

function AccountCardImpl({ account, code, now }: Props) {
  const showToast = useStore((s) => s.showToast);
  const openDialog = useStore((s) => s.openDialog);
  const refreshAccounts = useStore((s) => s.refreshAccounts);
  const groups = useStore((s) => s.groups);
  const hideCodes = useStore((s) => s.settings.hide_codes === "true");
  const [revealed, setRevealed] = useState(false);
  const [advancing, setAdvancing] = useState(false);

  const group = account.groupId ? groups.find((g) => g.id === account.groupId) : null;
  const primary = primaryLabel(account.issuer, account.accountName);
  const secondary = secondaryLabel(account.issuer, account.accountName);
  const remaining = code && code.type === "totp" ? Math.max(0, code.expiresAt - now) : 0;
  const blur = hideCodes && !revealed;

  const copy = async () => {
    try {
      await ipc.copyCode(account.id);
      showToast("Code copied", "success");
    } catch (e) {
      showToast(errorMessage(e), "danger");
    }
  };

  const toggleFavorite = async () => {
    try {
      await ipc.accountUpdate(account.id, { favorite: !account.favorite });
      await refreshAccounts();
    } catch (e) {
      showToast(errorMessage(e), "danger");
    }
  };

  const advanceHotp = async () => {
    setAdvancing(true);
    try {
      const next = await ipc.hotpAdvance(account.id);
      // Update just this code in the store.
      useStore.setState({ codes: { ...useStore.getState().codes, [account.id]: next } });
    } catch (e) {
      showToast(errorMessage(e), "danger");
    } finally {
      window.setTimeout(() => setAdvancing(false), 400);
    }
  };

  const codeText = code ? groupCode(code.code) : "••• •••";

  return (
    <div className="ck-card" data-favorite={account.favorite || undefined}>
      <ServiceIcon issuer={account.issuer} accountName={account.accountName} />

      <div className="ck-card__info">
        <div className="ck-card__labels">
          <span className="ck-card__primary" title={primary}>
            {primary}
          </span>
          {secondary && (
            <span className="ck-card__secondary" title={secondary}>
              {secondary}
            </span>
          )}
        </div>
        {group && <span className="ck-tag">{group.name}</span>}
      </div>

      <button
        className={`ck-card__code${blur ? " is-hidden" : ""}`}
        onClick={blur ? () => setRevealed(true) : copy}
        title={blur ? "Click to reveal" : "Click to copy"}
        aria-label={blur ? "Reveal code" : `Copy code for ${primary}`}
      >
        <span className="ck-card__digits">{codeText}</span>
      </button>

      <div className="ck-card__meter">
        {code?.type === "hotp" ? (
          <button
            className={`ck-hotp${advancing ? " is-spinning" : ""}`}
            onClick={advanceHotp}
            title="Generate next code"
            aria-label="Generate next HOTP code"
          >
            <RefreshCw size={16} />
            <span className="ck-hotp__counter">#{code.counter}</span>
          </button>
        ) : (
          code && (
            <CountdownRing key={code.expiresAt} remaining={remaining} period={code.period || 30} />
          )
        )}
      </div>

      <div className="ck-card__actions">
        <button
          className={`ck-iconbtn ck-card__star${account.favorite ? " is-on" : ""}`}
          onClick={toggleFavorite}
          aria-pressed={account.favorite}
          aria-label={account.favorite ? "Remove from favorites" : "Add to favorites"}
          title={account.favorite ? "Unfavorite" : "Favorite"}
        >
          <Star size={16} fill={account.favorite ? "currentColor" : "none"} />
        </button>

        <Menu
          align="right"
          trigger={(p) => (
            <button className="ck-iconbtn" aria-label="Account options" title="Options" {...p}>
              <MoreHorizontal size={18} />
            </button>
          )}
          items={[
            { label: "Copy code", icon: <Copy size={15} />, onClick: copy },
            {
              label: account.favorite ? "Remove favorite" : "Add to favorites",
              icon: <Star size={15} />,
              onClick: toggleFavorite,
            },
            {
              label: "Edit",
              icon: <Pencil size={15} />,
              onClick: () => openDialog({ kind: "edit", accountId: account.id }),
            },
            {
              label: "Reveal secret",
              icon: <Eye size={15} />,
              onClick: () => openDialog({ kind: "reveal", accountId: account.id }),
            },
            { separator: true },
            {
              label: "Delete",
              icon: <Trash2 size={15} />,
              danger: true,
              onClick: () => openDialog({ kind: "delete", accountId: account.id }),
            },
          ]}
        />
      </div>
    </div>
  );
}

export const AccountCard = memo(AccountCardImpl);
