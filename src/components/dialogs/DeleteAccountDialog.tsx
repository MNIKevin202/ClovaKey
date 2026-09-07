import { useStore } from "@/state/store";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { primaryLabel } from "@/lib/format";
import { errorMessage } from "@/lib/types";
import * as ipc from "@/lib/ipc";

export function DeleteAccountDialog({ accountId }: { accountId: string }) {
  const account = useStore((s) => s.accounts.find((a) => a.id === accountId));
  const closeDialog = useStore((s) => s.closeDialog);
  const refreshAll = useStore((s) => s.refreshAll);
  const showToast = useStore((s) => s.showToast);

  if (!account) {
    closeDialog();
    return null;
  }

  const label = primaryLabel(account.issuer, account.accountName);

  return (
    <ConfirmDialog
      title={`Delete ${label}?`}
      danger
      confirmLabel="Delete account"
      message={
        <>
          This permanently removes <strong>{label}</strong> and its secret from ClovaKey. If this is
          your only copy, make sure you can still sign in to that service first. This can’t be
          undone.
        </>
      }
      onClose={closeDialog}
      onConfirm={async () => {
        try {
          await ipc.accountDelete(account.id);
          await refreshAll();
          showToast("Account deleted");
        } catch (e) {
          showToast(errorMessage(e), "danger");
        }
      }}
    />
  );
}
