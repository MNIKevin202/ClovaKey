import { useRef, useState } from "react";
import { Camera, FileKey, Image as ImageIcon, KeySquare, Link2 } from "lucide-react";
import { Modal } from "@/components/Modal";
import { useStore } from "@/state/store";
import { fileToBase64 } from "@/lib/image";
import { errorMessage } from "@/lib/types";
import * as ipc from "@/lib/ipc";

const GoogleGlyph = () => (
  <svg width="22" height="22" viewBox="0 0 24 24" aria-hidden="true">
    <path
      fill="currentColor"
      d="M12 11v2.8h4a3.9 3.9 0 0 1-1.7 2.6l2.7 2.1A8 8 0 0 0 20 12c0-.6-.05-1.1-.15-1.6z"
    />
    <path
      fill="currentColor"
      opacity="0.55"
      d="M6.3 14.3A5 5 0 0 1 12 7a5 5 0 0 1 3.2 1.2l2.4-2.3A8 8 0 0 0 4.5 8.6z"
    />
    <path
      fill="currentColor"
      opacity="0.8"
      d="M12 20a8 8 0 0 0 5.5-2l-2.7-2.1a5 5 0 0 1-8.5-1.6L3.6 16.6A8 8 0 0 0 12 20z"
    />
  </svg>
);

export function AddAccountDialog() {
  const openDialog = useStore((s) => s.openDialog);
  const closeDialog = useStore((s) => s.closeDialog);
  const refreshAccounts = useStore((s) => s.refreshAccounts);
  const refreshCodes = useStore((s) => s.refreshCodes);
  const showToast = useStore((s) => s.showToast);
  const fileRef = useRef<HTMLInputElement>(null);
  const [note, setNote] = useState<string | null>(null);

  const onImagePicked = async (file: File) => {
    setNote(null);
    try {
      const b64 = await fileToBase64(file);
      const outcome = await ipc.scanQrImage(b64);
      if (outcome.kind === "otpauth" && outcome.account) {
        await Promise.all([refreshAccounts(), refreshCodes()]);
        showToast(`Added ${outcome.account.issuer ?? outcome.account.accountName}`, "success");
        closeDialog();
      } else if (outcome.kind === "migration") {
        openDialog({ kind: "google", seed: { kind: "image", data: b64 } });
      } else {
        setNote("No authenticator QR code was found in that image.");
      }
    } catch (e) {
      setNote(errorMessage(e));
    }
  };

  const options = [
    {
      icon: <Camera size={20} />,
      title: "Scan QR code",
      desc: "Use your camera to scan a setup QR code.",
      onClick: () => openDialog({ kind: "scan" }),
    },
    {
      icon: <ImageIcon size={20} />,
      title: "Import a QR image",
      desc: "Choose a screenshot or photo of a QR code.",
      onClick: () => fileRef.current?.click(),
    },
    {
      icon: <Link2 size={20} />,
      title: "Paste a setup link",
      desc: "Paste an otpauth:// link you copied.",
      onClick: () => openDialog({ kind: "uri" }),
    },
    {
      icon: <KeySquare size={20} />,
      title: "Enter a setup key",
      desc: "Type the secret key and details by hand.",
      onClick: () => openDialog({ kind: "manual" }),
    },
    {
      icon: <GoogleGlyph />,
      title: "Import from Google Authenticator",
      desc: "Move your accounts across using an export QR.",
      onClick: () => openDialog({ kind: "google" }),
      highlight: true,
    },
  ];

  return (
    <Modal
      title="Add an account"
      subtitle="Every method keeps your secret on this device."
      onClose={closeDialog}
    >
      <div className="ck-optiongrid">
        {options.map((o) => (
          <button
            key={o.title}
            className={`ck-option${o.highlight ? " is-highlight" : ""}`}
            onClick={o.onClick}
          >
            <span className="ck-option__icon">{o.icon}</span>
            <span className="ck-option__text">
              <span className="ck-option__title">{o.title}</span>
              <span className="ck-option__desc">{o.desc}</span>
            </span>
          </button>
        ))}
      </div>

      {note && (
        <p className="ck-note ck-note--warn">
          <FileKey size={15} />
          {note}
        </p>
      )}

      <input
        ref={fileRef}
        type="file"
        accept="image/*"
        hidden
        onChange={(e) => {
          const file = e.target.files?.[0];
          if (file) void onImagePicked(file);
          e.target.value = "";
        }}
      />
    </Modal>
  );
}
