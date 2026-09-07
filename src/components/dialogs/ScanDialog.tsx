import { useCallback, useEffect, useRef, useState } from "react";
import jsQR from "jsqr";
import { Camera, Image as ImageIcon, Link2 } from "lucide-react";
import { Modal } from "@/components/Modal";
import { useStore } from "@/state/store";
import { fileToBase64 } from "@/lib/image";
import { errorMessage } from "@/lib/types";
import * as ipc from "@/lib/ipc";

type Phase = "idle" | "starting" | "scanning" | "error";

export function ScanDialog() {
  const closeDialog = useStore((s) => s.closeDialog);
  const openDialog = useStore((s) => s.openDialog);
  const refreshAccounts = useStore((s) => s.refreshAccounts);
  const refreshCodes = useStore((s) => s.refreshCodes);
  const showToast = useStore((s) => s.showToast);

  const videoRef = useRef<HTMLVideoElement>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const rafRef = useRef<number>(0);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const handledRef = useRef(false);

  const [phase, setPhase] = useState<Phase>("idle");
  const [error, setError] = useState<string | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);

  const stop = useCallback(() => {
    cancelAnimationFrame(rafRef.current);
    streamRef.current?.getTracks().forEach((t) => t.stop());
    streamRef.current = null;
  }, []);

  const handleContent = useCallback(
    async (content: string) => {
      if (handledRef.current) return;
      handledRef.current = true;
      stop();
      try {
        const outcome = await ipc.scanQrText(content);
        if (outcome.kind === "otpauth" && outcome.account) {
          await Promise.all([refreshAccounts(), refreshCodes()]);
          showToast(`Added ${outcome.account.issuer ?? outcome.account.accountName}`, "success");
          closeDialog();
        } else if (outcome.kind === "migration") {
          openDialog({ kind: "google", seed: { kind: "text", data: content } });
        } else {
          setError("That QR code isn’t an authenticator setup code.");
          setPhase("error");
          handledRef.current = false;
        }
      } catch (e) {
        setError(errorMessage(e));
        setPhase("error");
        handledRef.current = false;
      }
    },
    [stop, refreshAccounts, refreshCodes, showToast, closeDialog, openDialog],
  );

  const tick = useCallback(() => {
    const video = videoRef.current;
    if (!video || video.readyState !== video.HAVE_ENOUGH_DATA) {
      rafRef.current = requestAnimationFrame(tick);
      return;
    }
    const canvas = (canvasRef.current ??= document.createElement("canvas"));
    canvas.width = video.videoWidth;
    canvas.height = video.videoHeight;
    const ctx = canvas.getContext("2d", { willReadFrequently: true });
    if (ctx && canvas.width > 0) {
      ctx.drawImage(video, 0, 0, canvas.width, canvas.height);
      const img = ctx.getImageData(0, 0, canvas.width, canvas.height);
      const found = jsQR(img.data, img.width, img.height, { inversionAttempts: "attemptBoth" });
      if (found?.data) {
        void handleContent(found.data);
        return;
      }
    }
    rafRef.current = requestAnimationFrame(tick);
  }, [handleContent]);

  const start = useCallback(async () => {
    setPhase("starting");
    setError(null);
    handledRef.current = false;
    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: "environment" },
        audio: false,
      });
      streamRef.current = stream;
      if (videoRef.current) {
        videoRef.current.srcObject = stream;
        await videoRef.current.play();
      }
      setPhase("scanning");
      rafRef.current = requestAnimationFrame(tick);
    } catch (e) {
      setError(
        "Camera unavailable. Grant camera access, or import a QR image / paste a link instead.",
      );
      setPhase("error");
      void e;
    }
  }, [tick]);

  useEffect(() => () => stop(), [stop]);

  const onImagePicked = async (file: File) => {
    try {
      const b64 = await fileToBase64(file);
      const outcome = await ipc.scanQrImage(b64);
      if (outcome.kind === "otpauth" && outcome.account) {
        await Promise.all([refreshAccounts(), refreshCodes()]);
        showToast("Account added", "success");
        closeDialog();
      } else if (outcome.kind === "migration") {
        openDialog({ kind: "google", seed: { kind: "image", data: b64 } });
      } else {
        setError("No authenticator QR code was found in that image.");
      }
    } catch (e) {
      setError(errorMessage(e));
    }
  };

  return (
    <Modal
      title="Scan a QR code"
      subtitle="Frames are decoded on your device — nothing is uploaded."
      onClose={() => {
        stop();
        closeDialog();
      }}
    >
      <div className="ck-scanner">
        <div className="ck-scanner__stage">
          <video ref={videoRef} className="ck-scanner__video" playsInline muted />
          {phase === "scanning" && <div className="ck-scanner__reticle" aria-hidden="true" />}
          {phase !== "scanning" && (
            <div className="ck-scanner__placeholder">
              <Camera size={34} />
              {phase === "starting" ? (
                <p>Starting camera…</p>
              ) : (
                <button className="ck-btn ck-btn--primary" onClick={() => void start()}>
                  Start camera
                </button>
              )}
            </div>
          )}
        </div>

        {error && <p className="ck-note ck-note--error">{error}</p>}

        <div className="ck-scanner__fallback">
          <button className="ck-btn ck-btn--ghost" onClick={() => fileRef.current?.click()}>
            <ImageIcon size={16} />
            Import image
          </button>
          <button
            className="ck-btn ck-btn--ghost"
            onClick={() => {
              stop();
              openDialog({ kind: "uri" });
            }}
          >
            <Link2 size={16} />
            Paste link
          </button>
        </div>
      </div>

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
