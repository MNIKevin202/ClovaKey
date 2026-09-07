// Display formatting helpers.

/** Group OTP digits for legibility: 6→"482 193", 8→"4821 9370". */
export function groupCode(code: string): string {
  const n = code.length;
  if (n === 6) return `${code.slice(0, 3)} ${code.slice(3)}`;
  if (n === 7) return `${code.slice(0, 4)} ${code.slice(4)}`;
  if (n === 8) return `${code.slice(0, 4)} ${code.slice(4)}`;
  const half = Math.ceil(n / 2);
  return `${code.slice(0, half)} ${code.slice(half)}`;
}

/** A primary label for an account: issuer if present, else account name. */
export function primaryLabel(issuer: string | null, accountName: string): string {
  return issuer && issuer.trim() ? issuer : accountName;
}

/** A secondary label: account name (only if it differs from the primary). */
export function secondaryLabel(issuer: string | null, accountName: string): string | null {
  if (!issuer || !issuer.trim()) return null;
  return accountName || null;
}

export function pluralize(n: number, one: string, many?: string): string {
  return n === 1 ? one : (many ?? `${one}s`);
}
