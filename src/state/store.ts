import { create } from "zustand";
import * as ipc from "@/lib/ipc";
import type { Account, GeneratedCode, Group, TimeStatus, VaultStatus } from "@/lib/types";

export type ViewId = "auth" | "favorites" | "groups" | "import" | "settings";

export interface GoogleSeed {
  kind: "text" | "image";
  data: string;
}

export type Dialog =
  | { kind: "none" }
  | { kind: "add" }
  | { kind: "manual" }
  | { kind: "uri" }
  | { kind: "scan" }
  | { kind: "google"; seed?: GoogleSeed }
  | { kind: "edit"; accountId: string }
  | { kind: "reveal"; accountId: string }
  | { kind: "delete"; accountId: string }
  | { kind: "backupCreate" }
  | { kind: "restore" };

export interface Toast {
  id: number;
  message: string;
  tone: "default" | "success" | "danger";
}

interface AppStore {
  // Lifecycle
  status: VaultStatus | null;
  ready: boolean;
  timeStatus: TimeStatus | null;

  // Data
  accounts: Account[];
  groups: Group[];
  codes: Record<string, GeneratedCode>;
  settings: Record<string, string>;

  // UI
  view: ViewId;
  search: string;
  activeGroupId: string | null;
  dialog: Dialog;
  toast: Toast | null;
  now: number;

  // Derived helpers
  setView: (v: ViewId) => void;
  setSearch: (s: string) => void;
  setActiveGroup: (id: string | null) => void;
  openDialog: (d: Dialog) => void;
  closeDialog: () => void;
  showToast: (message: string, tone?: Toast["tone"]) => void;

  // Async actions
  init: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  setup: (passphrase: string | null) => Promise<void>;
  unlock: (passphrase: string | null) => Promise<void>;
  lock: () => Promise<void>;
  onLockedEvent: () => void;
  refreshAll: () => Promise<void>;
  refreshAccounts: () => Promise<void>;
  refreshGroups: () => Promise<void>;
  refreshCodes: () => Promise<void>;
  regenerateIfExpired: () => Promise<void>;
  applySetting: (key: string, value: string) => Promise<void>;
  tick: () => void;
}

const nowSec = () => Math.floor(Date.now() / 1000);

let toastCounter = 0;

export const useStore = create<AppStore>((set, get) => ({
  status: null,
  ready: false,
  timeStatus: null,
  accounts: [],
  groups: [],
  codes: {},
  settings: {},
  view: "auth",
  search: "",
  activeGroupId: null,
  dialog: { kind: "none" },
  toast: null,
  now: nowSec(),

  setView: (v) => set({ view: v, search: "" }),
  setSearch: (s) => set({ search: s }),
  setActiveGroup: (id) => set({ activeGroupId: id }),
  openDialog: (d) => set({ dialog: d }),
  closeDialog: () => set({ dialog: { kind: "none" } }),
  showToast: (message, tone = "default") => {
    const id = ++toastCounter;
    set({ toast: { id, message, tone } });
    window.setTimeout(() => {
      if (get().toast?.id === id) set({ toast: null });
    }, 2200);
  },

  init: async () => {
    const status = await ipc.vaultStatus();
    const settings = await ipc.settingsGetAll();
    let timeStatus: TimeStatus | null = null;
    try {
      timeStatus = await ipc.timeStatus();
    } catch {
      timeStatus = null;
    }
    set({ status, settings, timeStatus, ready: true });
    if (
      status.initialized &&
      status.locked &&
      status.protection === "keychain" &&
      settings.lock_on_start !== "true"
    ) {
      // Frictionless keychain unlock on launch (unless the user opted into
      // lock-on-start). Failure just leaves the lock screen showing.
      try {
        await get().unlock(null);
      } catch {
        /* remain locked */
      }
    } else if (status.initialized && !status.locked) {
      await get().refreshAll();
    }
  },

  refreshStatus: async () => {
    const status = await ipc.vaultStatus();
    set({ status });
  },

  setup: async (passphrase) => {
    const status = await ipc.vaultSetup(passphrase);
    set({ status });
    await get().refreshAll();
  },

  unlock: async (passphrase) => {
    const status = await ipc.vaultUnlock(passphrase);
    set({ status });
    await get().refreshAll();
  },

  lock: async () => {
    await ipc.vaultLock();
    set({
      status: {
        ...(get().status ?? { initialized: true, protection: null }),
        locked: true,
      } as VaultStatus,
      codes: {},
    });
  },

  onLockedEvent: () => {
    const status = get().status;
    if (status) set({ status: { ...status, locked: true }, codes: {} });
  },

  refreshAll: async () => {
    await Promise.all([get().refreshAccounts(), get().refreshGroups()]);
    await get().refreshCodes();
  },

  refreshAccounts: async () => {
    const accounts = await ipc.accountsList();
    set({ accounts });
  },

  refreshGroups: async () => {
    const groups = await ipc.groupsList();
    set({ groups });
  },

  refreshCodes: async () => {
    if (get().status?.locked) return;
    try {
      const list = await ipc.codesGenerateAll();
      const codes: Record<string, GeneratedCode> = {};
      for (const c of list) codes[c.accountId] = c;
      set({ codes, now: nowSec() });
    } catch {
      // Locked or transient; ignore.
    }
  },

  regenerateIfExpired: async () => {
    const { codes, status } = get();
    if (status?.locked) return;
    const now = nowSec();
    const anyExpired = Object.values(codes).some(
      (c) => c.type === "totp" && c.expiresAt > 0 && c.expiresAt <= now,
    );
    if (anyExpired) await get().refreshCodes();
  },

  applySetting: async (key, value) => {
    await ipc.settingsSet(key, value);
    set({ settings: { ...get().settings, [key]: value } });
  },

  tick: () => set({ now: nowSec() }),
}));
