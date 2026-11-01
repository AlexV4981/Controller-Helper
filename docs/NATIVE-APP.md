# Linux desktop window

Controller Studio 0.5.0 uses a GTK 3 window with the system WebKitGTK 4.1 engine.
The controller engine and emulator configuration logic remain Rust. The interface,
including linked diagrams and the 3D view, is the same local bundled interface.
Opening Studio does not start Firefox or another browser.

Install the Mint packages `python3-gi` and `gir1.2-webkit2-4.1` from Software Manager.
Build with `cargo build --release --locked`, then run
`/usr/bin/python3 scripts/install-desktop.py` as your desktop user. The installer
keeps backups under `~/.local/state/controller-studio/desktop-backups`. No games,
saves, controller mappings or emulator preferences are part of this installation.

The engine executable and `controller-studio-native.py` must be installed together
in `~/.local/bin`. Running the engine normally opens the native window. Developer
and background commands keep their command-line behavior. For development, run
`/usr/bin/python3 native/app.py --backend target/release/controller-studio`.

The GTK application presents the existing window on repeated launch. It waits for
an engine with the matching app identity, version and settings home. An unrelated
or older service on the port shows an error instead of being loaded or replaced.
The window only navigates within the exact loopback origin. New windows, downloads
and permission requests are denied. Session cookies use an ephemeral WebKit context;
controller preferences remain in the engine's existing local storage. No native
file-access JavaScript bridge is exposed.

Closing the window leaves the background controller engine running, so queued
changes and automatic controller support continue. Unsaved edits trigger a discard
prompt, and the window waits while a save is in progress. The Help dialog's
**Close the helper** action stops the engine and closes the native window. Reopening
then starts the engine again. No sudo permissions or privileged service are added.

Checks: `python3 tests/native_app.py`, `python3 tests/native_dispatch.py` (after a
release build), existing Rust/JavaScript/API regressions, and an actual Mint window
launch/close/reopen check. Automated tests use disposable settings homes. Hardware
and game acceptance claims remain those documented in DS-DISCOVERY.md.

## Installed Mint check

On 2026-10-31, the installed app opened its GTK window in Mint 22.3 under
QEMU, displayed the existing Xbox controller and rendered the interactive 3D
view. Launching it again presented the same window. Closing after a temporary
unsaved edit displayed the discard warning; the test edit was discarded without
applying it. Existing system WebKitGTK packages were sufficient. Native hardware
gameplay acceptance remains separate from this desktop-window check.
