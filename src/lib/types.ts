// Types mirroring the Rust command surface (serde camelCase output).

export type OtpType = "totp" | "hotp";
export type Algorithm = "SHA1" | "SHA256" | "SHA512";

export interface Account {
  id: string;
  issuer: string | null;
  accountName: string;
  // OtpConfig is flattened into the account.
  type: OtpType;
  algorithm: Algorithm;
  digits: number;
  period: number;
  counter: number;
  groupId: string | null;
  favorite: boolean;
  sortOrder: number;
  icon: string | null;
  createdAt: number;
  updatedAt: number;
}

export interface GeneratedCode {
  accountId: string;
  code: string;
  type: OtpType;
  digits: number;
  period: number;
  expiresAt: number;
  secondsRemaining: number;
  counter: number;
}

export interface Group {
  id: string;
  name: string;
  sortOrder: number;
}

export interface VaultStatus {
  initialized: boolean;
  locked: boolean;
  protection: "keychain" | "passphrase" | null;
}

export interface PreviewItem {
  index: number;
  issuer: string | null;
  accountName: string;
  type: OtpType;
  algorithm: Algorithm;
  digits: number;
  period: number;
  counter: number;
  duplicate: boolean;
}

export interface BatchProgress {
  received: number;
  total: number;
  complete: boolean;
}

export interface ImportSummary {
  imported: number;
  skipped: number;
  groupsCreated: number;
}

export interface ScanOutcome {
  kind: "otpauth" | "migration" | "unknown";
  account: Account | null;
}

export interface RevealedSecret {
  uri: string;
  secret: string;
  qrSvg: string;
}

export interface TimeStatus {
  unixTime: number;
  ok: boolean;
  reason: string | null;
}

export interface SecurityCapabilities {
  platform: string;
  biometricAvailable: boolean;
  biometricKind: string | null;
}

export interface ManualAccountInput {
  issuer?: string | null;
  accountName: string;
  secret: string;
  type: OtpType;
  algorithm: Algorithm;
  digits: number;
  period: number;
  counter?: number;
  groupId?: string | null;
  favorite?: boolean;
  icon?: string | null;
}

export interface AccountPatch {
  // Present with a value sets it; present with null clears it; omitted leaves it.
  issuer?: string | null;
  accountName?: string;
  groupId?: string | null;
  favorite?: boolean;
  icon?: string | null;
  sortOrder?: number;
}

export interface ImportSelection {
  index: number;
  issuer?: string | null;
  accountName?: string | null;
  groupId?: string | null;
  favorite?: boolean;
}

/** The `{ code, message }` shape thrown by every command on failure. */
export interface AppError {
  code: string;
  message: string;
}

export function isAppError(e: unknown): e is AppError {
  return (
    typeof e === "object" &&
    e !== null &&
    "code" in e &&
    "message" in e &&
    typeof (e as AppError).message === "string"
  );
}

export function errorMessage(e: unknown): string {
  if (isAppError(e)) return e.message;
  if (e instanceof Error) return e.message;
  return String(e);
}
