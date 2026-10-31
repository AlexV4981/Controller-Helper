# Manifest audit against the newer local source

Audit date: 2026-10-31. This is an audit and revised implementation contract, not a claim that P0–P3 have been implemented or accepted on the laptop.

## Baselines

- Requested instructions: [PROJECT_MANIFEST.md](https://github.com/AlexV4981/Controller-Helper/blob/MelonDS-Patch-%26-QOL/PROJECT_MANIFEST.md), GitHub blob `b775a3edaf9de02b7148726b459581e4f7f897e3`.
- The branch currently declares **0.2.0**, not the newer local version. Its manifest identifies baseline commit `29c5693a4611550bfe4a0c7b1008baa555534a7c`.
- The working local source declares **0.3.2** and contains substantial existing uncommitted work. Do not check out the old branch over it or replace current files with its copies.
- Last recorded Mint installation is the 0.3.1 helper, binary SHA-256 `3dceff3db37f40a7a1034481529932da166e013c843d6c20d5464971f3431462`. A local 0.3.2 build is not evidence that 0.3.2 is installed or running on Mint. Recheck the target before deployment.
- Reference copies of the retrieved manifest and six baseline files are outside the application repository, in the workspace's `manifest-audit/` directory.

## What to keep, skip, or adapt

| Manifest area | Current evidence | Decision |
| --- | --- | --- |
| DS discovery without source edits | `src/games.rs::catalog/find/resolve` still use bundled JSON, two DS game IDs and exact filenames. | **Implement P0.** This limitation remains. |
| Diagnose melonDS's own file picker | The fixed Studio list does not establish a melonDS file-picker failure. The user now reports games load. | **Separate diagnosis.** Confirm which application has the discovery symptom before changing Flatpak access or emulator behavior. |
| DS per-game config activation | `games::ds_launch_args` now uses `flatpak run --command=env … XDG_CONFIG_HOME=… melonDS`, with an argument-boundary regression test. | **Preserve the local fix.** Do not restore the older outer `--env` launch command. Deployment still needs checking. |
| DS display and patched Mario 64 DS | Current code preserves aspect settings; the managed Rev 1 widescreen copy has revision and SHA-256 checks. | **Preserve.** Do not reset to the old display defaults, overwrite ROMs, or make every ASME variant use the managed-patch checksum. |
| Keyboard/shared DS use | Shared profile generation and keyboard-only launch handling already exist. Isolated configs still require Studio/game-specific launch. | **Retain and regression-test.** Do not claim an isolated config follows a ROM chosen inside an already-open melonDS instance. |
| Dolphin per-game layouts | Native game settings and separate Classic, sideways and Nunchuk profiles already exist. | **Keep working paths.** Dynamic DS discovery must not replace or reset them. |
| Queued configuration writes | Current source includes persisted revisions, pending application, emulator-running guards and application status. | **Keep and extend tests.** Do not rebuild this from the 0.2.0 implementation. |
| Controller continuity and USB diagram | Stable device identities, saved player roles, physical port labels and priority selection already exist. | **Preserve existing preferences.** A library refresh must not reassign players. |
| Existing controller visuals | Three.js assets, linked mapping diagrams and accessible mapping controls already exist. | **Do not rebuild them.** P1 is a desktop-shell change, not a new controller renderer. |
| Standalone desktop shell | Current Cargo dependencies and launch path still implement a browser/local-HTTP application. | **Still needed, P1 only after P0's native gate.** Validate Mint/WebKitGTK support before choosing packaging details. |
| Saved bindings without a running server | Profiles are persistent emulator files. New device monitoring and reapplication are separate behavior. | **Preserve direct use.** Make monitoring optional without falsely promising new-device remapping with no process running. |
| General adapters and optional pre-launch preview | No general dynamic multi-emulator library or suppression preferences are present. | **Defer to P2/P3**, following the requested phase gates. Never require a preview for direct emulator startup. |

## Required corrections to the implementation plan

1. **Include `src/runtime.rs`.** It is newer than the referenced branch and also uses the fixed catalog. Keep its public-window-title observation advisory. A window title or four-character DS code cannot distinguish two ROM variants with the same title/code; ambiguous matches must remain unknown rather than selecting a profile incorrectly.
2. **Separate identity, console code, display title and location.** A content identity must survive renaming without conflating two ASME revisions, a patched copy, or two same-titled homebrew games. Preserve the existing ASME/CLJE mappings through a tested legacy-alias migration; do not silently discard or duplicate controller preferences.
3. **Retain settings for removed games.** Current `storage::load_settings` validates saved game references with `games::find`. Replacing that with only the currently present files would make deleting a ROM invalidate otherwise valid preferences. Keep validated unavailable identity records, disable their launches and preserve their settings for later rediscovery.
4. **Do not scan or regenerate every ROM on controller polls.** Refresh a bounded index explicitly or with controlled background work. Cache validated content digests; represent pending identity work explicitly. Generate a discovered DS game's profile on demand, keeping its safe configuration path registered before applying or restoring it.
5. **Authorize exact registered config paths.** Do not replace the existing allowlist with a broad `controller-studio-games/*` prefix or a caller-supplied path. Validate index ownership/schema, generated IDs, size bounds, root confinement and symlink behavior before trusting persisted records. Handle replacement/race cases as safe failures.
6. **Accept valid trimmed/patched ROMs deliberately.** The installed widescreen ROM is trimmed. Declared cartridge capacity need not equal file length; validate the header and referenced ranges without accidentally rejecting the working managed copy. Commercial IDs must not be required for homebrew. Synthetic parser fixtures are not playable-ROM evidence.
7. **Keep ROM/save policy separate from controller identity.** Renaming a ROM may affect its ROM-adjacent save filename even if its mapping identity is stable. Do not rename, move or overwrite saves as part of scanner migration. Shared custom save directories can also make identical basenames collide; report the ambiguity instead of silently choosing a save.
8. **Preserve guarded isolated launches.** DS mode, built-in BIOS restrictions, safe save-path validation, existing backups, undo, keyboard/touch fallback and emulator-running write deferral remain required. Do not broaden filesystem access merely because a ROM is absent from Studio's list.
9. **Record native results separately.** User-reported native loading success is useful evidence; it is not a fresh test of dynamic discovery, every binding or multiplayer. Earlier QEMU checks also found a melonDS fullscreen Alt+F4 exit fault; F11 then File → Quit exited cleanly. Preserve this as a separate issue to confirm natively, rather than attributing it to the scanner or reverting a working game patch.

## Updated execution order

1. Identify whether the missing-game symptom is in Studio, melonDS, or both; inspect the currently installed helper version and configured library locations.
2. P0 A: add bounded DS parsing/indexing and tests for valid additional games, malformed headers, trimmed files, Unicode/spaces, rename, duplicates, missing files and symlink/race boundaries.
3. P0 B: integrate the same validated index with APIs, model, exact storage authorization and on-demand profiles; migrate legacy IDs without losing current settings. Preserve shared direct-emulator behavior and advisory runtime detection.
4. Run automated regressions, then the manifest's **native Mint P0 acceptance gate** using two actual additional playable DS ROMs and the connected Xbox controllers. Keep PlayStation/Switch behavior explicitly hardware-untested unless devices are available.
5. Only after that gate passes: P1 desktop shell, P2 adapters, then P3 optional previews. Do not merge or deploy a later phase on the strength of synthetic tests or QEMU alone.

## Verification performed for this audit

- `cargo test --locked --offline`: **47 passed**, no failures.
- `cargo clippy --locked --offline --all-targets -- -D warnings`: passed; two pre-existing warnings remain in the vendored `tiny_http` dependency.
- `node --test tests/input.test.mjs tests/ports.test.mjs tests/diagrams.test.mjs`: **3 test files passed**.
- Read the remote manifest and relevant 0.2.0 files through the GitHub connector; compared their contracts with the newer local source and existing installation receipts.
- No emulator configuration, ROM, save, GitHub branch, or installed helper was changed during this audit; no disk mounts or installation operations were performed. No new native acceptance result is claimed.

## Clarified live melonDS symptom (2026-10-31)

The user confirms that Studio detects an open melonDS process but cannot identify
its loaded game and keeps suggesting a launch. This is **runtime identification**,
not a report that melonDS cannot open ROMs or that new ROMs are absent from the
selector. Dynamic filesystem discovery is a separate manifest requirement and
cannot recover the current game from a version/FPS-only window title.

Upstream `src/frontend/qt_sdl/EmuThread.cpp` constructs the running title as
`[%d/%.0f] melonDS <version>` and the idle title as `melonDS <version>`.
Source: https://github.com/melonDS-emu/melonDS/blob/master/src/frontend/qt_sdl/EmuThread.cpp
(lines 424–438 inspected on 2026-10-31). This agrees with the previously observed
1.1 window. No loaded ROM identity is present in that signal. Do not use the last
played entry or recent-file list as proof of the current game.

Local source correction: distinguish an open melonDS session with unavailable game
identity from no running game; explain shared direct-use controls and optional
isolated layouts; remove the unconditional Studio-launch instruction from shared
save success. Window-title matching no longer claims it verified an active saved
profile. Unknown sessions do not overwrite the editing selection or unsaved drafts.
These changes do not implement live game identification, dynamic scanning, or a
native shell. They have not been installed or verified on native Mint yet.
