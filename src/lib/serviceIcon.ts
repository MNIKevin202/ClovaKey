// Deterministic, privacy-preserving service icons.
//
// We never call an external icon API. A recognizable brand tint is used for
// well-known issuers; everything else gets a stable hue derived from the name.
// The result always looks intentional, even with no match.

export interface IconStyle {
  initials: string;
  bg: string;
  fg: string;
}

// Recognizable accent colours for common issuers (no logos, just a tint).
const BRAND: Record<string, string> = {
  google: "#4285F4",
  gmail: "#EA4335",
  youtube: "#FF0000",
  github: "#7d8590",
  gitlab: "#FC6D26",
  microsoft: "#00A4EF",
  outlook: "#0072C6",
  azure: "#0089D6",
  amazon: "#FF9900",
  aws: "#FF9900",
  apple: "#9aa0a6",
  icloud: "#3693F3",
  facebook: "#1877F2",
  meta: "#1877F2",
  instagram: "#E4405F",
  whatsapp: "#25D366",
  twitter: "#1DA1F2",
  x: "#61748a",
  discord: "#5865F2",
  slack: "#611f69",
  dropbox: "#0061FF",
  steam: "#66c0f4",
  twitch: "#9146FF",
  paypal: "#0070BA",
  stripe: "#635BFF",
  coinbase: "#0052FF",
  binance: "#F3BA2F",
  kraken: "#5741D9",
  linkedin: "#0A66C2",
  reddit: "#FF4500",
  npm: "#CB3837",
  cloudflare: "#F38020",
  digitalocean: "#0080FF",
  heroku: "#79589F",
  notion: "#787774",
  figma: "#A259FF",
  atlassian: "#2684FF",
  bitwarden: "#175DDC",
  proton: "#6D4AFF",
  protonmail: "#6D4AFF",
  nintendo: "#E60012",
  epic: "#787878",
  epicgames: "#787878",
  ubisoft: "#0070FF",
  ea: "#FF4747",
  sony: "#0072CE",
  playstation: "#0070D1",
};

function normalize(s: string): string {
  return s
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

function hashHue(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 360;
  return h;
}

function initialsFor(label: string): string {
  const words = label.trim().split(/\s+/).filter(Boolean);
  if (words.length === 0) return "?";
  if (words.length === 1) {
    const w = words[0];
    return (w.length >= 2 ? w.slice(0, 2) : w).toUpperCase();
  }
  return (words[0][0] + words[1][0]).toUpperCase();
}

/** Compute a stable icon style for an issuer/account label. */
export function iconStyle(issuer: string | null, accountName: string): IconStyle {
  const label = (issuer && issuer.trim()) || accountName || "?";
  const norm = normalize(label);
  const initials = initialsFor(label);

  // Exact or first-token brand match.
  const token = norm.split(" ")[0];
  const brand = BRAND[norm] ?? BRAND[token];
  if (brand) {
    return { initials, bg: brand, fg: readableText(brand) };
  }

  const hue = hashHue(norm || label);
  const bg = `hsl(${hue} 42% 46%)`;
  return { initials, bg, fg: "#ffffff" };
}

// Choose black/white text for contrast against a hex colour.
function readableText(hex: string): string {
  const c = hex.replace("#", "");
  if (c.length !== 6) return "#ffffff";
  const r = parseInt(c.slice(0, 2), 16);
  const g = parseInt(c.slice(2, 4), 16);
  const b = parseInt(c.slice(4, 6), 16);
  const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
  return luminance > 0.62 ? "#10201a" : "#ffffff";
}
