// Typed wrappers around Tauri commands. This is the ONLY place the frontend
// talks to the Rust core. Secrets are never returned here except by the
// explicit, re-authenticated `accountReveal` call.

import { invoke } from "@tauri-apps/api/core";
import type {
  Account,
  AccountPatch,
  BatchProgress,
  GeneratedCode,
  Group,
  ImportSelection,
  ImportSummary,
  ManualAccountInput,
  PreviewItem,
  RevealedSecret,
  ScanOutcome,
  SecurityCapabilities,
  TimeStatus,
  VaultStatus,
} from "./types";

// ── Vault lifecycle ──────────────────────────────────────────────────────────
export const vaultStatus = () => invoke<VaultStatus>("vault_status");
export const vaultSetup = (passphrase: string | null) =>
  invoke<VaultStatus>("vault_setup", { passphrase });
export const vaultUnlock = (passphrase: string | null) =>
  invoke<VaultStatus>("vault_unlock", { passphrase });
export const vaultLock = () => invoke<void>("vault_lock");
export const vaultSetPassphrase = (passphrase: string) =>
  invoke<void>("vault_set_passphrase", { passphrase });
export const vaultRemovePassphrase = (passphrase: string) =>
  invoke<void>("vault_remove_passphrase", { passphrase });
export const vaultChangePassphrase = (oldPassphrase: string, newPassphrase: string) =>
  invoke<void>("vault_change_passphrase", { oldPassphrase, newPassphrase });
export const noteActivity = () => invoke<void>("note_activity");

// ── Accounts & codes ─────────────────────────────────────────────────────────
export const accountsList = () => invoke<Account[]>("accounts_list");
export const codesGenerateAll = () => invoke<GeneratedCode[]>("codes_generate_all");
export const codeGenerate = (id: string) => invoke<GeneratedCode>("code_generate", { id });
export const hotpAdvance = (id: string) => invoke<GeneratedCode>("hotp_advance", { id });
export const accountAddManual = (input: ManualAccountInput) =>
  invoke<Account>("account_add_manual", { input });
export const accountAddUri = (uri: string) => invoke<Account>("account_add_uri", { uri });
export const accountUpdate = (id: string, patch: AccountPatch) =>
  invoke<Account>("account_update", { id, patch });
export const accountDelete = (id: string) => invoke<void>("account_delete", { id });
export const accountReorder = (ids: string[]) => invoke<void>("account_reorder", { ids });
export const accountReveal = (id: string, passphrase: string | null) =>
  invoke<RevealedSecret>("account_reveal", { id, passphrase });

// ── Groups ───────────────────────────────────────────────────────────────────
export const groupsList = () => invoke<Group[]>("groups_list");
export const groupCreate = (name: string) => invoke<Group>("group_create", { name });
export const groupRename = (id: string, name: string) =>
  invoke<void>("group_rename", { id, name });
export const groupDelete = (id: string) => invoke<void>("group_delete", { id });

// ── Clipboard ──────────────────────────────────────────────────────────────
export const copyCode = (id: string) => invoke<void>("copy_code", { id });

// ── Scan (single account) ────────────────────────────────────────────────────
export const scanQrText = (text: string) => invoke<ScanOutcome>("scan_qr_text", { text });
export const scanQrImage = (imageBase64: string) =>
  invoke<ScanOutcome>("scan_qr_image", { imageBase64 });

// ── Google import wizard ─────────────────────────────────────────────────────
export const googleImportBegin = () => invoke<string>("google_import_begin");
export const googleImportAddText = (sessionId: string, text: string) =>
  invoke<BatchProgress>("google_import_add_text", { sessionId, text });
export const googleImportAddImage = (sessionId: string, imageBase64: string) =>
  invoke<BatchProgress>("google_import_add_image", { sessionId, imageBase64 });
export const googleImportPreview = (sessionId: string) =>
  invoke<PreviewItem[]>("google_import_preview", { sessionId });
export const googleImportCommit = (sessionId: string, selections: ImportSelection[]) =>
  invoke<ImportSummary>("google_import_commit", { sessionId, selections });
export const googleImportCancel = (sessionId: string) =>
  invoke<void>("google_import_cancel", { sessionId });

// ── Backup & restore ─────────────────────────────────────────────────────────
export const backupCreate = (path: string, password: string) =>
  invoke<void>("backup_create", { path, password });
export const backupRestore = (path: string, password: string, importAll: boolean) =>
  invoke<ImportSummary>("backup_restore", { path, password, importAll });

// ── Settings & data ──────────────────────────────────────────────────────────
export const settingsGetAll = () => invoke<Record<string, string>>("settings_get_all");
export const settingsSet = (key: string, value: string) =>
  invoke<void>("settings_set", { key, value });
export const dataWipe = (passphrase: string | null) => invoke<void>("data_wipe", { passphrase });

// ── Environment ──────────────────────────────────────────────────────────────
export const timeStatus = () => invoke<TimeStatus>("time_status");
export const securityCapabilities = () =>
  invoke<SecurityCapabilities>("security_capabilities");
