// Low-end devices can't keep up with live `backdrop-filter` while scrolling.
// `deviceMemory` is Chromium-only (bucketed: 0.25 … 8 GB); Safari leaves it
// undefined, so iPhones keep full effects. Flags <html> for aero-theme.css.

type NavigatorWithMemory = Navigator & { deviceMemory?: number }

function isLowEndDevice(): boolean {
  const nav = navigator as NavigatorWithMemory
  const lowMemory = nav.deviceMemory !== undefined && nav.deviceMemory <= 4
  const fewCores = nav.hardwareConcurrency !== undefined && nav.hardwareConcurrency <= 2
  return lowMemory || fewCores
}

export function applyEffectsLevel(): void {
  if (isLowEndDevice()) document.documentElement.dataset.effects = "reduced"
}
