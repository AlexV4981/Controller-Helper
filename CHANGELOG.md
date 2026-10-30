## 0.2.0

Add isolated per-game controller layouts, shared fallback/inheritance, and guarded game launch for the imported Wii/DS library. DS uses original SDL joystick indexes and highest saved player priority, corrects trigger/inverted-axis bindings, enables scalable OpenGL presentation, and adds F11/fullscreen launch. Dolphin starts fullscreen. Harden game paths, TOML validation, configuration startup gating, and asynchronous editor drafts.

## 0.1.6

Keep Dolphin input enabled across window focus changes. Preserve per-game Classic/sideways/Nunchuk selection, saved player identities, and graphics/speed preferences.

## 0.1.5

Show a live USB socket view for each player, saved unplugged assignments, USB hub paths, and confirmed physical side labels. Bluetooth controllers are distinguished from USB sockets.

## 0.1.4

Persist primary/secondary roles using controller serial, paired-device identity, or USB port instead of changing event numbers. Reserve disconnected player slots, retain custom mappings during upgrade, and show primary/secondary roles in the interface.

## 0.1.3

Classic A also wakes the underlying emulated Wii Remote; extension-only inputs cannot activate an inactive Bluetooth device.

## 0.1.2

Use Dolphin 2609a SDL3 directional Button S/E/W/N input names and model names for the selected device, matching its detected controller dropdown. Explicit SDL2 aliases remain in input expressions.

## 0.1.1

Fix controller identity collisions, stale player ports, Cemu player compaction, duplicate managed INI keys, interrupted-save recovery, and stalled HTTP bodies. Add per-game Wii presets and Classic keyboard fallback.

# Changelog

## 0.1.0 — 2026-10-29

- Detect supported USB/Bluetooth gamepads through SDL and remember players.
- Match Xbox/Nintendo printed labels, offer position matching and editable controls.
- Add live 3D controller view, press-to-assign, mouse/keyboard alternatives, and two-player presets.
- Save profiles for puNES, melonDS, Azahar, Dolphin and Cemu with private backups and guarded restoration.
- Bundle the offline interface, per-user launcher and automatic startup.
