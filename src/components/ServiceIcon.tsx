import { iconStyle } from "@/lib/serviceIcon";

interface Props {
  issuer: string | null;
  accountName: string;
  size?: number;
}

/** A polished, deterministic issuer tile (brand tint + initials). */
export function ServiceIcon({ issuer, accountName, size = 40 }: Props) {
  const { initials, bg, fg } = iconStyle(issuer, accountName);
  return (
    <span
      className="ck-svcicon"
      aria-hidden="true"
      style={{
        width: size,
        height: size,
        // A subtle vertical sheen over the brand/hue colour.
        background: `linear-gradient(160deg, color-mix(in srgb, ${bg} 88%, white 12%), ${bg})`,
        color: fg,
        fontSize: size * 0.36,
      }}
    >
      {initials}
    </span>
  );
}
