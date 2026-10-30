/** Learn only a fresh deliberate input, never a held button or idle stick drift. */
export function nextPress(current, previous) {
  return Object.entries(current).find(([control, value]) => value > .7 && (previous[control] || 0) < .4)?.[0] ?? null;
}
