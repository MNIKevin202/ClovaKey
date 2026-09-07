import type { CSSProperties } from "react";

interface Props {
  remaining: number;
  period: number;
  size?: number;
}

/**
 * A smooth TOTP countdown ring. The depletion is driven by a CSS animation
 * (GPU-friendly, no per-frame JS) synced to the current window via a negative
 * animation-delay; the parent re-keys it each window so it never drifts.
 *
 * The depleting ring + the numeric label are both non-colour cues, so the
 * status is legible without relying on colour alone.
 */
export function CountdownRing({ remaining, period, size = 38 }: Props) {
  const stroke = 3;
  const r = (size - stroke) / 2;
  const circumference = 2 * Math.PI * r;
  const elapsed = Math.max(0, period - remaining);
  const urgent = remaining <= 5;

  const progressStyle: CSSProperties = {
    ["--ring-c" as string]: `${circumference}px`,
    animationDuration: `${period}s`,
    animationDelay: `-${elapsed}s`,
  };

  return (
    <div
      className="ck-ring"
      style={{ width: size, height: size }}
      role="timer"
      aria-label={`${remaining} seconds until the code refreshes`}
    >
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} aria-hidden="true">
        <circle
          className="ck-ring__track"
          cx={size / 2}
          cy={size / 2}
          r={r}
          strokeWidth={stroke}
          fill="none"
        />
        <circle
          className={`ck-ring__progress${urgent ? " is-urgent" : ""}`}
          cx={size / 2}
          cy={size / 2}
          r={r}
          strokeWidth={stroke}
          strokeLinecap="round"
          fill="none"
          strokeDasharray={circumference}
          style={progressStyle}
        />
      </svg>
      <span className={`ck-ring__num${urgent ? " is-urgent" : ""}`}>{remaining}</span>
    </div>
  );
}
