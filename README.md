# Controller Helper

Plug in a controller, pick a player, and change the controls in a 3D view. The app window is called Controller Studio.

Built for Linux Mint and the Flatpak versions of Dolphin, melonDS, Azahar, puNES and Cemu. It runs locally as your normal user.

## Build and run

You need Rust with edition 2024 support, SDL2 and a browser. Install the emulators separately.

```sh
cargo build --release --locked
./target/release/controller-studio
```

To keep the binary in your home folder:

```sh
mkdir -p ~/.local/bin
install -m 755 target/release/controller-studio ~/.local/bin/controller-studio
~/.local/bin/controller-studio
```

No sudo needed. Pair Bluetooth controllers in Mint's Bluetooth settings first. USB controllers need a data cable.

## Controls

Pick a controller and assign its player number. Player 1 is primary. The USB view shows which socket belongs to each player; you can label the physical ports yourself.

**Match labels** maps Xbox A to Nintendo A. **Match positions** maps by where the buttons sit. PlayStation controllers show their own symbols and the Nintendo action they control.

Select an action and choose a button, or use **Press a button to assign**. You can also drag an action onto the 3D controller. Drag empty space to rotate and scroll to zoom. The regular button list works without the 3D view.

Click **Use these controls** to save. Changes wait while an emulator is open. Close the emulator before applying a new layout.

## Per-game layouts

Choose a game under **Edit controls for**. Saving only changes that game's layout for the selected controller. **Use shared layout** removes the override. Unsaved edits stay separate when switching games.

Dolphin loads each game's profile automatically. Mario Kart Wii and Brawl use Classic controls, New Super Mario Bros. Wii uses sideways controls, and Galaxy uses Nunchuk controls.

Launch DS games with **Launch game** in Studio to use their custom layout. Opening melonDS directly uses the shared layout. DS uses the lowest numbered connected player, then keyboard and mouse if no controller is connected.

Dolphin starts fullscreen. melonDS has a screen-layout selector: one large screen, a large top screen with a small touchscreen, or both screens side by side. Tab swaps screens and F11 toggles fullscreen. Mario 64 DS defaults to one large screen; Bowser's Inside Story keeps both visible. Keyboard bindings and mouse touchscreen controls are kept.

The paired diagrams show the emulated controller on the left and your real controller on the right. Green means assigned. Select either side to highlight matching controls in gold; selecting a button does not remap it. Use the binding selector or press-to-assign to change it.

Studio follows a recognized game in Dolphin or melonDS's X11 window title. The active-game card shows what it detected. Unknown titles and multiple games stay explicit; choose a game manually when detection is unavailable. Turn off **Follow the active game** to browse layouts yourself. Game detection needs `xprop` and an X11 session. It does not read game memory or guess from an old command line. The game catalog, controls and last detected game stay available offline.

Saved changes wait until the whole emulator exits, including Dolphin's game-list window. A centered dialog explains this and changes to **Successful** only after the matching saved revision applies. Queued manual changes survive restarting Studio, even with auto-apply off. File errors stay errors and can be retried with **Use these controls**.

For New Super Mario Bros. Wii: Xbox A jumps (Wii 2), X runs (Wii 1), B bubbles (Wii A), LT maps Wii B, RB shakes, and the right stick tilts. Keyboard: X jump, Z run, C bubble/A, V B, Space shake, Q/E tilt left/right, R/F tilt forward/back, arrows move, Enter pause. Existing custom shake/tilt mappings are kept during the old-preset upgrade.

The current game list is in `src/games.json`. It contains 11 Wii games and 2 DS games and expects matching files under `~/Games/Nintendo/Wii` and `~/Games/Nintendo/DS`. No games, BIOS files or keys are included or downloaded.

DS game launches support standard DS mode with built-in BIOS. Save paths must be ROM-adjacent or absolute. External BIOS, DSi mode and ambiguous relative save paths are rejected; open the shared emulator for those setups.

## What was tested

Two Xbox One USB controllers were tested on the original laptop. PlayStation and Switch handling is implemented, but physical controllers were not available to test. USB roles stay with the port when a controller has no unique serial. Bluetooth roles use the paired device identity when available.

DS and 3DS are single-player per instance. Touchscreens, microphones and Wii motion controls may still need the keyboard or mouse. Game compatibility and speed depend on your hardware and emulator settings. Native gameplay still needs a retest after a controller or display change. X11 fixture tests verify game switching and queued saves; they do not prove game compatibility or native graphics performance.

## Backups

Settings and recovery files are in `~/.local/state/controller-studio/`. Emulator profiles are backed up before replacement. **Undo last setup** restores the previous profiles if they have not changed since the backup.

The helper only writes known emulator config paths. It rejects symlinks and waits while emulators are running. The local server listens on `127.0.0.1` and checks the request origin and session cookie. It does not change sudo permissions or install drivers.

These are config backups, so keep your own backups of saves and games.

## Development

```sh
cargo test --locked
cargo clippy --all-targets -- -D warnings
node --test tests/input.test.mjs tests/ports.test.mjs
```

`--inspect` lists controllers. `--export-plan --home PATH` prints proposed config files without writing them. `--apply-once --home PATH` applies to a test home. An alternate home shows a preview badge. `--background` starts without opening a browser.

The 3D renderer and web UI are bundled. SDL2 is loaded at runtime, so SDL development headers are not needed to compile. Third-party licenses are included under `vendor/`.

## Mario 64 DS widescreen

The widescreen patch requires USA Rev 1 / v1.1. Studio prefers the separately verified `Super Mario 64 DS (USA) (Rev 1) (Widescreen).nds` copy when present and checks its SHA-256 before launching. The original games stay intact. Launch it through Studio or the dedicated widescreen shortcut to use the 16:9 game config. See [patch notes](docs/widescreen.md) for the source and checks.
