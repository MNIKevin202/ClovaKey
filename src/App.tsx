import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { platform } from "@tauri-apps/plugin-os";
import { useStore } from "@/state/store";
import { applyAllSettings } from "@/lib/theme";
import { SEARCH_INPUT_ID } from "@/components/SearchBar";
import * as ipc from "@/lib/ipc";

import { Sidebar } from "@/components/Sidebar";
import { Toast } from "@/components/Toast";
import { Authenticator } from "@/views/Authenticator";
import { Favorites } from "@/views/Favorites";
import { GroupsView } from "@/views/GroupsView";
import { ImportView } from "@/views/ImportView";
import { Settings } from "@/views/Settings";
import { LockScreen } from "@/views/LockScreen";
import { FirstRun } from "@/views/FirstRun";

import { AddAccountDialog } from "@/components/dialogs/AddAccountDialog";
import { ManualEntryDialog } from "@/components/dialogs/ManualEntryDialog";
import { UriDialog } from "@/components/dialogs/UriDialog";
import { ScanDialog } from "@/components/dialogs/ScanDialog";
import { GoogleImportWizard } from "@/components/dialogs/GoogleImportWizard";
import { EditAccountDialog } from "@/components/dialogs/EditAccountDialog";
import { RevealSecretDialog } from "@/components/dialogs/RevealSecretDialog";
import { DeleteAccountDialog } from "@/components/dialogs/DeleteAccountDialog";
import { BackupCreateDialog } from "@/components/dialogs/BackupCreateDialog";
import { RestoreBackupDialog } from "@/components/dialogs/RestoreBackupDialog";

function DialogHost() {
  const dialog = useStore((s) => s.dialog);
  switch (dialog.kind) {
    case "add":
      return <AddAccountDialog />;
    case "manual":
      return <ManualEntryDialog />;
    case "uri":
      return <UriDialog />;
    case "scan":
      return <ScanDialog />;
    case "google":
      return <GoogleImportWizard seed={dialog.seed} />;
    case "edit":
      return <EditAccountDialog accountId={dialog.accountId} />;
    case "reveal":
      return <RevealSecretDialog accountId={dialog.accountId} />;
    case "delete":
      return <DeleteAccountDialog accountId={dialog.accountId} />;
    case "backupCreate":
      return <BackupCreateDialog />;
    case "restore":
      return <RestoreBackupDialog />;
    default:
      return null;
  }
}

function MainView() {
  const view = useStore((s) => s.view);
  switch (view) {
    case "favorites":
      return <Favorites />;
    case "groups":
      return <GroupsView />;
    case "import":
      return <ImportView />;
    case "settings":
      return <Settings />;
    default:
      return <Authenticator />;
  }
}

export default function App() {
  const ready = useStore((s) => s.ready);
  const status = useStore((s) => s.status);
  const settings = useStore((s) => s.settings);

  // One-time init + live-updates subscription + code ticker.
  useEffect(() => {
    try {
      document.documentElement.dataset.os = platform();
    } catch {
      /* non-Tauri context */
    }
    void useStore.getState().init();

    const unlisten = listen("clovakey://locked", () => {
      useStore.getState().onLockedEvent();
    });

    const ticker = window.setInterval(() => {
      const s = useStore.getState();
      s.tick();
      void s.regenerateIfExpired(); // no-ops when locked
    }, 1000);

    return () => {
      window.clearInterval(ticker);
      void unlisten.then((f) => f());
    };
  }, []);

  // Apply theme / density / motion whenever settings change.
  useEffect(() => {
    applyAllSettings(settings);
  }, [settings]);

  // Global keyboard shortcuts + activity pings (only when unlocked).
  useEffect(() => {
    const unlocked = status?.initialized && !status.locked;
    if (!unlocked) return;

    const mod = (e: KeyboardEvent) => e.metaKey || e.ctrlKey;
    const onKey = (e: KeyboardEvent) => {
      const s = useStore.getState();
      if (mod(e) && e.key.toLowerCase() === "n") {
        e.preventDefault();
        s.openDialog({ kind: "add" });
      } else if (mod(e) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        if (s.view !== "auth" && s.view !== "favorites") s.setView("auth");
        window.setTimeout(() => document.getElementById(SEARCH_INPUT_ID)?.focus(), 0);
      } else if (mod(e) && e.key === ",") {
        e.preventDefault();
        s.setView("settings");
      } else if (mod(e) && e.key.toLowerCase() === "l") {
        e.preventDefault();
        void s.lock();
      } else if (e.key === "Escape" && s.dialog.kind === "none" && s.search) {
        s.setSearch("");
      }
    };

    let last = 0;
    const onActivity = () => {
      const t = Date.now();
      if (t - last > 20000) {
        last = t;
        void ipc.noteActivity();
      }
    };

    window.addEventListener("keydown", onKey);
    window.addEventListener("mousedown", onActivity);
    window.addEventListener("keydown", onActivity);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("mousedown", onActivity);
      window.removeEventListener("keydown", onActivity);
    };
  }, [status?.initialized, status?.locked]);

  if (!ready) {
    return <div className="ck-boot" />;
  }

  if (!status?.initialized) {
    return (
      <>
        <FirstRun />
        <DialogHost />
        <Toast />
      </>
    );
  }

  if (status.locked) {
    return <LockScreen />;
  }

  return (
    <div className="ck-app">
      <Sidebar />
      <main className="ck-main">
        <MainView />
      </main>
      <DialogHost />
      <Toast />
    </div>
  );
}
