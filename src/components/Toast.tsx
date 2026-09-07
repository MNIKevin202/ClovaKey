import { Check, ClipboardCheck } from "lucide-react";
import { useStore } from "@/state/store";

/** Transient confirmation toast (e.g. "Code copied"). */
export function Toast() {
  const toast = useStore((s) => s.toast);
  if (!toast) return null;
  return (
    <div className={`ck-toast ck-toast--${toast.tone}`} role="status" aria-live="polite">
      {toast.tone === "success" ? <ClipboardCheck size={16} /> : <Check size={16} />}
      <span>{toast.message}</span>
    </div>
  );
}
