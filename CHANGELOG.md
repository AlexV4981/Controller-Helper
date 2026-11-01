## 0.5.1

Refuse a conflicting controller-engine port before initializing settings. Reuse an
engine still starting when retrying the native window, and reject redirects during
the local health check. Include the missing literal-path launch regression test.

## 0.5.0

Open Controller Studio in its own Linux desktop window instead of a Firefox tab.
Repeated launches present the existing window. Keep the background engine running
when the window closes, warn before discarding edits, and retain queued changes.
Show startup and connection failures in the app. Restrict the embedded view to the
local interface and keep all existing controller settings.

## 0.4.0 (native gameplay acceptance pending)

Discover DS ROMs in subfolders without adding source entries. Keep content identities and layouts after renames or removal, distinguish duplicate titles, and report invalid files. Generate dynamic DS profiles when requested and retain shared controls for direct emulator use.

Add an exact content-ID observer and a pinned source patch for melonDS 1.1 to publish its loaded cartridge identity in its window title. Stock melonDS still cannot provide this signal. The installed Mint integration was rebuilt in its KDE 6.10 SDK and verified under QEMU with both existing DS games, including switching inside melonDS. Native gameplay acceptance remains pending.

## 0.3.2

Clarify that melonDS can be open without exposing its loaded game to Studio. Keep direct play on shared controls and describe isolated DS launches as optional. Stop claiming a detected title proves which controls are active. Automatic DS game identification and dynamic library scanning remain unfinished.

Set the DS per-game configuration directory inside the Flatpak sandbox so the selected screen layout and controls actually take effect.

## 0.3.1

Preserve DS game aspect settings when applying controllers. Prefer the separately verified Mario 64 DS USA Rev 1 widescreen copy without replacing the original game. Verify the patched ROM hash before launch.

## 0.3.0

Add linked emulated/physical controller diagrams and X11 active-game following. Keep per-game drafts separate during game switches. Persist queued saves and acknowledge their applied revisions before showing success. Separate file errors from emulator-running waits. Add large DS screen layouts and fix overlapping sideways jump/bubble controls, missing B and keyboard alternatives. Leave the incompatible Mario 64 DS widescreen patch unapplied.

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
