// Applies the user's theme + density + reduced-motion preferences to <html>.

export type ThemeSetting = "system" | "light" | "dark";

export function applyTheme(theme: string | undefined) {
  const root = document.documentElement;
  if (theme === "light" || theme === "dark") {
    root.setAttribute("data-theme", theme);
  } else {
    // "system" — remove the attribute and let prefers-color-scheme decide.
    root.removeAttribute("data-theme");
  }
}

export function applyDensity(density: string | undefined) {
  document.documentElement.setAttribute(
    "data-density",
    density === "compact" ? "compact" : "comfortable",
  );
}

export function applyReducedMotion(reduce: string | undefined) {
  document.documentElement.toggleAttribute("data-reduce-motion", reduce === "true");
}

export function applyAllSettings(settings: Record<string, string>) {
  applyTheme(settings.theme);
  applyDensity(settings.density);
  applyReducedMotion(settings.reduce_motion);
}
