import { Modal } from "@/components/Modal";

const LIBS: { name: string; license: string; use: string }[] = [
  { name: "Tauri", license: "MIT / Apache-2.0", use: "Application shell" },
  { name: "React", license: "MIT", use: "User interface" },
  { name: "RustCrypto (hmac, sha1, sha2)", license: "MIT / Apache-2.0", use: "OTP + fingerprints" },
  { name: "chacha20poly1305", license: "Apache-2.0 / MIT", use: "Authenticated encryption" },
  { name: "argon2", license: "MIT / Apache-2.0", use: "Passphrase key derivation" },
  { name: "rusqlite / SQLite", license: "MIT / Public Domain", use: "Encrypted vault store" },
  { name: "keyring", license: "MIT / Apache-2.0", use: "OS keychain access" },
  { name: "rqrr", license: "MIT / Apache-2.0", use: "QR decoding" },
  { name: "qrcode", license: "MIT / Apache-2.0", use: "QR generation" },
  { name: "jsQR", license: "Apache-2.0", use: "Camera QR decoding" },
  { name: "lucide", license: "ISC", use: "Interface icons" },
  { name: "zustand", license: "MIT", use: "UI state" },
];

export function LicensesDialog({ onClose }: { onClose: () => void }) {
  return (
    <Modal
      title="Open-source licenses"
      subtitle="ClovaKey is built with these open-source projects."
      onClose={onClose}
    >
      <ul className="ck-licenses">
        {LIBS.map((l) => (
          <li key={l.name} className="ck-licenses__row">
            <div>
              <span className="ck-licenses__name">{l.name}</span>
              <span className="ck-licenses__use">{l.use}</span>
            </div>
            <span className="ck-badge">{l.license}</span>
          </li>
        ))}
      </ul>
    </Modal>
  );
}
