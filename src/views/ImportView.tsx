import { ArrowRight, DownloadCloud, PlusCircle, ShieldCheck } from "lucide-react";
import { useStore } from "@/state/store";

export function ImportView() {
  const openDialog = useStore((s) => s.openDialog);

  const cards = [
    {
      icon: <ShieldCheck size={22} />,
      title: "Import from Google Authenticator",
      desc: "Move all your accounts across using Google Authenticator’s “Export accounts” QR codes — including exports split across several codes.",
      cta: "Start Google import",
      onClick: () => openDialog({ kind: "google" }),
      highlight: true,
    },
    {
      icon: <PlusCircle size={22} />,
      title: "Add a single account",
      desc: "Scan a QR code, import a QR image, paste an otpauth:// link, or type a setup key by hand.",
      cta: "Add account",
      onClick: () => openDialog({ kind: "add" }),
    },
    {
      icon: <DownloadCloud size={22} />,
      title: "Restore a ClovaKey backup",
      desc: "Bring back accounts from an encrypted .clovakey backup file created on this or another computer.",
      cta: "Restore backup",
      onClick: () => openDialog({ kind: "restore" }),
    },
  ];

  return (
    <div className="ck-view">
      <header className="ck-header" data-tauri-drag-region>
        <div className="ck-header__row">
          <h1 className="ck-header__title">Import</h1>
        </div>
      </header>
      <div className="ck-scroll">
        <p className="ck-lede">
          Every import is processed entirely on your device. QR codes are decoded locally and your
          secrets are never uploaded anywhere.
        </p>
        <div className="ck-importgrid">
          {cards.map((c) => (
            <button
              key={c.title}
              className={`ck-importcard${c.highlight ? " is-highlight" : ""}`}
              onClick={c.onClick}
            >
              <span className="ck-importcard__icon">{c.icon}</span>
              <span className="ck-importcard__title">{c.title}</span>
              <span className="ck-importcard__desc">{c.desc}</span>
              <span className="ck-importcard__cta">
                {c.cta}
                <ArrowRight size={15} />
              </span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
