import { type ReactNode, useEffect, useState } from "react";
import {
  ChevronDown,
  DatabaseZap,
  DownloadCloud,
  Fingerprint,
  KeyRound,
  Lock,
  ShieldCheck,
  Trash2,
  Upload,
} from "lucide-react";
import { getVersion } from "@tauri-apps/api/app";
import {
  disable as autostartDisable,
  enable as autostartEnable,
  isEnabled as autostartIsEnabled,
} from "@tauri-apps/plugin-autostart";
import { useStore } from "@/state/store";
import { applyDensity, applyReducedMotion, applyTheme } from "@/lib/theme";
import { errorMessage, type SecurityCapabilities } from "@/lib/types";
import * as ipc from "@/lib/ipc";
import { PassphraseDialog, type PassphraseMode } from "@/components/dialogs/PassphraseDialog";
import { WipeDataDialog } from "@/components/dialogs/WipeDataDialog";
import { LicensesDialog } from "@/components/dialogs/LicensesDialog";

function Section({
  title,
  icon,
  children,
}: {
  title: string;
  icon: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="ck-settings__section">
      <h2 className="ck-settings__heading">
        {icon}
        {title}
      </h2>
      <div className="ck-settings__card">{children}</div>
    </section>
  );
}

function Row({ label, desc, control }: { label: string; desc?: string; control: ReactNode }) {
  return (
    <div className="ck-setrow">
      <div className="ck-setrow__text">
        <span className="ck-setrow__label">{label}</span>
        {desc && <span className="ck-setrow__desc">{desc}</span>}
      </div>
      <div className="ck-setrow__control">{control}</div>
    </div>
  );
}

function Toggle({
  on,
  onChange,
  label,
}: {
  on: boolean;
  onChange: (v: boolean) => void;
  label: string;
}) {
  return (
    <button
      className={`ck-switch${on ? " is-on" : ""}`}
      role="switch"
      aria-checked={on}
      aria-label={label}
      onClick={() => onChange(!on)}
    >
      <span className="ck-switch__thumb" />
    </button>
  );
}

function Segmented<T extends string>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
}) {
  return (
    <div className="ck-segmented" role="group">
      {options.map((o) => (
        <button
          key={o.value}
          className={`ck-segmented__item${value === o.value ? " is-active" : ""}`}
          aria-pressed={value === o.value}
          onClick={() => onChange(o.value)}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

function Select({
  value,
  onChange,
  options,
}: {
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}) {
  return (
    <div className="ck-select ck-select--sm">
      <select value={value} onChange={(e) => onChange(e.target.value)}>
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
      <ChevronDown size={14} />
    </div>
  );
}

export function Settings() {
  const settings = useStore((s) => s.settings);
  const status = useStore((s) => s.status);
  const applySetting = useStore((s) => s.applySetting);
  const openDialog = useStore((s) => s.openDialog);

  const [autostart, setAutostart] = useState(false);
  const [version, setVersion] = useState("");
  const [caps, setCaps] = useState<SecurityCapabilities | null>(null);
  const [passphraseMode, setPassphraseMode] = useState<PassphraseMode | null>(null);
  const [wiping, setWiping] = useState(false);
  const [licenses, setLicenses] = useState(false);

  useEffect(() => {
    void (async () => {
      try {
        setAutostart(await autostartIsEnabled());
      } catch {
        /* autostart may be unavailable */
      }
      try {
        setVersion(await getVersion());
      } catch {
        /* ignore */
      }
      try {
        setCaps(await ipc.securityCapabilities());
      } catch {
        /* ignore */
      }
    })();
  }, []);

  const set = (key: string, value: string) => void applySetting(key, value);

  const toggleAutostart = async (v: boolean) => {
    try {
      if (v) await autostartEnable();
      else await autostartDisable();
      setAutostart(v);
    } catch (e) {
      useStore.getState().showToast(errorMessage(e), "danger");
    }
  };

  const hasPassphrase = status?.protection === "passphrase";

  return (
    <div className="ck-view">
      <header className="ck-header" data-tauri-drag-region>
        <div className="ck-header__row">
          <h1 className="ck-header__title">Settings</h1>
        </div>
      </header>

      <div className="ck-scroll ck-settings">
        <Section title="General" icon={<KeyRound size={16} />}>
          <Row
            label="Launch at login"
            desc="Open ClovaKey automatically when you sign in."
            control={<Toggle on={autostart} onChange={toggleAutostart} label="Launch at login" />}
          />
          <Row
            label="When the window is closed"
            desc="Keep ClovaKey running in the menu bar, or quit entirely."
            control={
              <Segmented
                value={settings.close_to_tray === "true" ? "tray" : "quit"}
                onChange={(v) => set("close_to_tray", v === "tray" ? "true" : "false")}
                options={[
                  { value: "tray", label: "Keep running" },
                  { value: "quit", label: "Quit" },
                ]}
              />
            }
          />
          <Row
            label="Show menu bar icon"
            desc="Quick access to open and lock ClovaKey. Restart to apply."
            control={
              <Toggle
                on={settings.show_tray !== "false"}
                onChange={(v) => set("show_tray", String(v))}
                label="Show menu bar icon"
              />
            }
          />
        </Section>

        <Section title="Appearance" icon={<KeyRound size={16} />}>
          <Row
            label="Theme"
            control={
              <Segmented
                value={(settings.theme as string) || "system"}
                onChange={(v) => {
                  set("theme", v);
                  applyTheme(v);
                }}
                options={[
                  { value: "system", label: "System" },
                  { value: "light", label: "Light" },
                  { value: "dark", label: "Dark" },
                ]}
              />
            }
          />
          <Row
            label="Density"
            desc="Comfortable spacing, or fit more accounts on screen."
            control={
              <Segmented
                value={(settings.density as string) || "comfortable"}
                onChange={(v) => {
                  set("density", v);
                  applyDensity(v);
                }}
                options={[
                  { value: "comfortable", label: "Comfortable" },
                  { value: "compact", label: "Compact" },
                ]}
              />
            }
          />
          <Row
            label="Hide codes until hovered"
            desc="Blur codes in the list until you hover or click them."
            control={
              <Toggle
                on={settings.hide_codes === "true"}
                onChange={(v) => set("hide_codes", String(v))}
                label="Hide codes until hovered"
              />
            }
          />
          <Row
            label="Reduce motion"
            desc="Minimize animations, including the countdown ring."
            control={
              <Toggle
                on={settings.reduce_motion === "true"}
                onChange={(v) => {
                  set("reduce_motion", String(v));
                  applyReducedMotion(String(v));
                }}
                label="Reduce motion"
              />
            }
          />
        </Section>

        <Section title="Security" icon={<ShieldCheck size={16} />}>
          <Row
            label="App passphrase"
            desc={
              hasPassphrase
                ? "A passphrase is required to unlock and generate codes."
                : "Your vault key is protected by the system keychain. Add a passphrase for a stronger, portable lock."
            }
            control={
              hasPassphrase ? (
                <div className="ck-btnrow">
                  <button
                    className="ck-btn ck-btn--ghost ck-btn--sm"
                    onClick={() => setPassphraseMode("change")}
                  >
                    Change
                  </button>
                  <button
                    className="ck-btn ck-btn--ghost ck-btn--sm"
                    onClick={() => setPassphraseMode("remove")}
                  >
                    Remove
                  </button>
                </div>
              ) : (
                <button
                  className="ck-btn ck-btn--primary ck-btn--sm"
                  onClick={() => setPassphraseMode("set")}
                >
                  <Lock size={14} />
                  Set passphrase
                </button>
              )
            }
          />
          <Row
            label="Auto-lock"
            desc="Lock ClovaKey automatically after a period of inactivity."
            control={
              <Select
                value={settings.auto_lock_secs || "0"}
                onChange={(v) => set("auto_lock_secs", v)}
                options={[
                  { value: "0", label: "Never" },
                  { value: "60", label: "1 minute" },
                  { value: "300", label: "5 minutes" },
                  { value: "900", label: "15 minutes" },
                  { value: "1800", label: "30 minutes" },
                ]}
              />
            }
          />
          <Row
            label="Biometric unlock"
            desc={
              caps?.biometricKind
                ? `${caps.biometricKind} support is planned for a future update.`
                : "Not available on this platform."
            }
            control={
              <span className="ck-pill">
                <Fingerprint size={14} />
                Coming soon
              </span>
            }
          />
        </Section>

        <Section title="Clipboard" icon={<KeyRound size={16} />}>
          <Row
            label="Clear copied codes"
            desc="Automatically clear a copied code from the clipboard. ClovaKey won’t erase anything you copy afterwards."
            control={
              <Select
                value={settings.clipboard_clear_secs || "30"}
                onChange={(v) => set("clipboard_clear_secs", v)}
                options={[
                  { value: "0", label: "Never" },
                  { value: "15", label: "After 15s" },
                  { value: "30", label: "After 30s" },
                  { value: "60", label: "After 60s" },
                ]}
              />
            }
          />
        </Section>

        <Section title="Backup & restore" icon={<DownloadCloud size={16} />}>
          <Row
            label="Create a backup"
            desc="Save all accounts to a portable, passphrase-encrypted .clovakey file you can restore on any computer."
            control={
              <button
                className="ck-btn ck-btn--primary ck-btn--sm"
                onClick={() => openDialog({ kind: "backupCreate" })}
              >
                <Upload size={14} />
                Create backup
              </button>
            }
          />
          <Row
            label="Restore from a backup"
            desc="Import accounts from a .clovakey file."
            control={
              <button
                className="ck-btn ck-btn--ghost ck-btn--sm"
                onClick={() => openDialog({ kind: "restore" })}
              >
                <DownloadCloud size={14} />
                Restore
              </button>
            }
          />
        </Section>

        <Section title="Data" icon={<DatabaseZap size={16} />}>
          <Row
            label="Import from Google Authenticator"
            control={
              <button
                className="ck-btn ck-btn--ghost ck-btn--sm"
                onClick={() => openDialog({ kind: "google" })}
              >
                Import
              </button>
            }
          />
          <Row
            label="Erase all data"
            desc="Permanently delete every account, group, and setting from this device."
            control={
              <button className="ck-btn ck-btn--danger ck-btn--sm" onClick={() => setWiping(true)}>
                <Trash2 size={14} />
                Erase…
              </button>
            }
          />
        </Section>

        <Section title="About" icon={<ShieldCheck size={16} />}>
          <div className="ck-about">
            <img src="/clovakey.png" alt="" width={56} height={56} />
            <div>
              <div className="ck-about__name">ClovaKey</div>
              <div className="ck-about__meta">
                Version {version || "0.1.0"} · {caps?.platform ?? "desktop"}
              </div>
              <div className="ck-about__meta">Offline-first · No account · No telemetry</div>
            </div>
          </div>
          <Row
            label="Automatic updates"
            desc="This build has no update endpoint configured, so ClovaKey never contacts a server. Your codes always work offline."
            control={<span className="ck-pill">Not configured</span>}
          />
          <Row
            label="Open-source licenses"
            control={
              <button className="ck-btn ck-btn--ghost ck-btn--sm" onClick={() => setLicenses(true)}>
                View
              </button>
            }
          />
        </Section>
      </div>

      {passphraseMode && (
        <PassphraseDialog mode={passphraseMode} onClose={() => setPassphraseMode(null)} />
      )}
      {wiping && <WipeDataDialog onClose={() => setWiping(false)} />}
      {licenses && <LicensesDialog onClose={() => setLicenses(false)} />}
    </div>
  );
}
