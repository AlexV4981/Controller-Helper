# Controller Studio — Linux Emulation Machine Development Manifest

> **Status:** Proposed plan; not an implementation report  
> **Target branch:** `MelonDS-Patch-&-QOL`  
> **Baseline inspected:** `29c5693a4611550bfe4a0c7b1008baa555534a7c` (branch initially identical to `main`)  
> **Prepared:** 2026-10-31  
> **Product scope:** Linux-first controller management for locally installed emulators, initially Linux Mint.

## 1. Product contract

Controller Studio is a **controller configuration companion**, not a mandatory game launcher and not a replacement for emulators or ES-DE.

The primary family workflow MUST remain:

1. Open Dolphin, melonDS, Azahar, puNES, or Cemu from the Linux desktop.
2. Select a game **inside that emulator**.
3. Play using already configured controller bindings.

Controller Studio is available separately to assign players, inspect connected gamepads, edit bindings, and restore configuration backups. The normal workflow must not require opening Controller Studio every time.

### Product decisions

- Keep **Rust** for controller/device, profile, validation, and storage logic.
- Keep the existing **JavaScript/Three.js** 3D controller view; make it a standalone Linux desktop window later, preferably using **Tauri 2**, rather than rebuilding the UI in another language.
- Keep Windows support **out of scope**.
- Use normal user permissions. No new drivers, sudo-at-runtime, accounts, or online services.
- Preserve the working emulator desktop shortcuts, saves, keyboard/mouse fallback, and existing user settings.
- Restrict game support to the configured **emulators**. Do not attempt universal Steam/native Linux game remapping.
- Any controller-preview/confirmation step is **optional** and must never prevent the normal direct-emulator workflow.

## 2. Observed baseline and actual limitation

The baseline `src/games.json` contains a fixed list of 11 Wii titles and **two DS titles** (`CLJE` and `ASME`) with exact expected filenames. `src/games.rs::catalog()` loads that bundled JSON rather than scanning the filesystem; `find()` and `resolve()` only accept its entries.

Other code assumes this fixed list:

- `src/main.rs`: `/api/state` returns catalogued games; `/api/save` and `/api/launch` validate IDs against the catalog.
- `src/profiles.rs`: isolated DS game configuration files are generated only for catalogued DS games.
- `src/storage.rs::allowed()`: only catalogued isolated DS config paths are allowlisted.
- `src/model.rs::game_bindings()`: game-specific bindings require a catalog lookup.
- `web/app.js`: the game selector consumes the catalog-derived API list.

**Important diagnostic distinction:** This explains why additional DS ROMs do not appear in **Controller Studio's game selector**. It does **not** establish that melonDS itself cannot see/open the ROMs. If melonDS's own file picker fails, reproduce and diagnose that as a separate issue (file type, path access, Flatpak permissions, or emulator behavior).

Directly opening melonDS currently uses its **shared** configuration. Studio-controlled DS launches use a per-game `XDG_CONFIG_HOME`, which is not automatically activated when a user selects a ROM inside the already-open emulator.

### Compatibility matrix (target contract)

| How game is opened | Shared bindings | Game-specific bindings | Studio pre-game prompt |
| --- | --- | --- | --- |
| Desktop → melonDS → choose ROM | **Yes** | **No**, not with present isolated-config mechanism | **No** |
| Studio → launch a particular DS ROM | Yes | **Yes**, via isolated config | Optional |
| Desktop → Dolphin → choose game | **Yes** | **Yes**, where Dolphin's native per-game profiles work | **No** |
| Studio → launch a particular supported game | Yes | Where supported by that emulator adapter | Optional |
| Desktop shortcut wrapped to start emulator | **Yes** | Only if the exact game is known beforehand | **Shared-controls preview only**, if enabled |

Never promise to intercept a game's selection inside an unmodified running emulator. Avoid process injection, fullscreen overlays, focus stealing, and undocumented emulator hooks.

## 3. Delivery order and gates

**Do not advance to the next phase until its acceptance gate passes on the target Linux Mint laptop.** Keep the existing tests and backup protections in every phase.

### P0 — Fix DS discovery and preserve direct melonDS use (highest priority)

**Goal:** Any ordinary, valid `.nds` ROM under a configured DS library folder can be discovered without editing `games.json`, while melonDS remains usable directly from its desktop shortcut.

Implementation checklist:

- [ ] Reproduce the reported failure separately in Studio's game selector and melonDS's own open-file dialog. Record which one fails.
- [ ] Introduce a runtime DS scanner for `~/Games/Nintendo/DS/` initially, with a configurable root later. Handle spaces, Unicode names, subfolders, empty folders, and refresh after files are added/removed.
- [ ] Inspect DS headers for plausible metadata and identity; do not rely on an exact filename. Use a persistent, collision-resistant game key (e.g. header game code **plus** content identity/cached digest), distinct from the visible display title. Two same-titled ROMs must not share custom settings accidentally; renaming a ROM should not silently lose its saved mapping.
- [ ] Reject malformed/unsupported ROMs with a readable reason. Do not treat file extensions alone as proof of validity, and do not require a known Nintendo game ID for arbitrary ROMs.
- [ ] Keep scanning confined to configured roots; do not traverse symlinks or path-escape locations. Bound scan time, file/header reads, and the number of records; do not scan entire home directories.
- [ ] Replace fixed-catalog lookups in `games.rs`, API handlers, `model.rs`, and DS profile generation with one validated, runtime library index.
- [ ] Update `storage.rs` so a generated internal game ID may authorize **only** its own known DS config path. Preserve existing allowlists, symlink checks, path traversal rejection, and atomic backup/restore behavior.
- [ ] Generate isolated DS profiles **on demand** for Studio-launched games rather than rewriting configurations for every scanned ROM on each controller poll.
- [ ] Keep the **shared melonDS controller layout** functional for direct emulator launch, including keyboard/touchscreen fallback. Do not claim isolated DS layouts apply to games opened inside melonDS.
- [ ] Preserve existing deliberate restrictions on isolated DS launches (built-in BIOS/standard DS mode and supported save paths), with actionable messages. Do not weaken validation to make discovery work.
- [ ] Report explicit reasons for missing ROMs, unreadable folders, incorrect Flatpak access, and invalid configuration.

**P0 acceptance gate**

- [ ] Add two different valid DS ROMs not named in `games.json`; both appear in Studio without recompiling.
- [ ] Remove/rename one ROM; refresh reflects the change and preserves identity where appropriate.
- [ ] Select an arbitrary discovered ROM; preview resolves its correct path. A Studio launch uses the correct isolated config and ROM argument.
- [ ] Open melonDS from the desktop independently; select any valid ROM and use shared controller/keyboard controls without Studio being open.
- [ ] Malformed ROM, symlink escape, duplicate title, and unsupported config produce safe failures without rewriting unrelated files.
- [ ] Existing `ASME`/Wii settings, player persistence, and restore behavior remain compatible.

### P1 — Native-feeling standalone Linux desktop application

**Goal:** Controller Studio opens as its own application window, not a browser tab.

- [ ] Package existing HTML/CSS/JavaScript/Three.js UI in a Tauri 2 desktop window; retain Rust logic and SDL input handling. Verify the Linux Mint WebKitGTK WebGL implementation supports the 3D renderer, with a usable 2D/list fallback.
- [ ] Preserve existing settings and backups through a versioned, tested migration; do not reset family controller roles.
- [ ] Move local HTTP operations to narrowly permitted Tauri commands incrementally, or securely bridge the existing local API while migrating; preserve session/origin protections until HTTP is removed.
- [ ] Add a desktop entry, icon, single-instance behavior, friendly first-run diagnostics, and a close/minimize policy that does not strand or unexpectedly stop emulator games.
- [ ] Provide a user-local install option (e.g. AppImage/user-level desktop entry) that does not require sudo at runtime; a system-wide `.deb` may separately require installation privileges.
- [ ] Keep original emulator desktop shortcuts intact and usable.

**P1 acceptance gate**

- [ ] Launch from Mint's menu/dock without opening a browser.
- [ ] Controller display, save, restore, rumble test, and 3D/list mapping all work.
- [ ] Close Studio; then open an emulator from the desktop and play with the saved shared bindings.
- [ ] No background server is required merely for an already saved static emulator configuration. If dynamic monitoring/auto-apply is later enabled, make it optional and clearly communicated.

### P2 — Arbitrary supported ROMs across the existing emulators

**Goal:** No per-title source-code edits for normal game discovery.

- [ ] Introduce an `EmulatorAdapter` interface: supported library roots/formats, identification, default controller style, shared/per-game profile capabilities, and optional launch command.
- [ ] Expand beyond DS to supported formats for Dolphin, Azahar, puNES, and Cemu **after verifying installed emulator versions and accepted file formats**. Do not equate extension detection with a claim that a game is playable.
- [ ] Keep one shared library index and stable game IDs; make display titles editable without changing identities.
- [ ] Use native emulator per-game settings where genuinely supported (notably Dolphin), while keeping shared fallback for other emulators.
- [ ] Treat Wii controller types as explicit metadata/presets or a user choice when uncertain (Classic, sideways, Nunchuk, motion); never silently guess a game's required motion capabilities.
- [ ] Keep ROMs and saves in their own folders; never download games, firmware, BIOS, or keys.
- [ ] Preserve per-user saves and configurations unless a separately designed shared-library option is explicitly enabled.

**P2 acceptance gate**

- [ ] Adding a supported ROM under an enabled emulator root makes it discoverable without rebuilding Studio.
- [ ] Each game uses the correct emulator adapter; unsupported/ambiguous files are clearly marked instead of launched blindly.
- [ ] Shared controls and emulator-native per-game profiles continue to work when emulators are opened directly.

### P3 — Optional confirmation and quality-of-life UI

**Goal:** A clear, single-monitor-friendly binding confirmation where Controller Studio knows the exact game **before launch**, without forcing family members through another launcher.

- [ ] Studio-controlled launch (or an explicitly game-specific desktop shortcut) can show connected controller, player assignment, action-to-button map, **Change controls**, and **Play**.
- [ ] Offer a persisted checkbox: **Don't show again for this game**. Store it per stable game ID (and account); expose **Show controls before launch** to reverse it. Consider re-showing if controller family or profile changes.
- [ ] Apply/save only validated bindings before launching. Hide/minimize the Studio window, start emulator normally/fullscreen, and restore Studio gracefully after emulator exit.
- [ ] For the normal `desktop → open emulator → select game inside emulator` workflow, do **not** show a per-game modal: the game is not known to Studio ahead of time. An optional wrapper may show a **shared controller summary before opening the emulator**, but not a game-specific preview.
- [ ] Keep all preview prompts opt-in or default-bypassable; never block direct emulator startup when Studio isn't running.
- [ ] Avoid injection into emulator windows, compositor-specific always-on-top overlays, and foreground polling of private emulator state.

**P3 acceptance gate**

- [ ] First Studio-controlled launch of a known game can display its preview; Play starts the correct emulator.
- [ ] Checking **Don't show again** suppresses later prompts for that game; resetting it restores the prompt.
- [ ] Another game can still have its own prompt preference.
- [ ] Direct desktop emulator launch remains unchanged, including after Studio exits or is uninstalled.

## 4. Engineering boundaries and invariants

1. **Emulator ownership:** Do not rewrite active emulator configuration files while the relevant emulator is running. Queue or defer changes, and explain that restart may be required.
2. **Safety:** Keep approved config-path allowlists, symlink/path traversal defenses, validation, bounded parsing, and safe command-argument construction. Do not execute commands derived from ROM titles.
3. **Backups:** Back up before modifications; provide predictable undo and failure rollback. Do not overwrite game saves as part of a controller configuration restore.
4. **Controller continuity:** Keep stable USB/Bluetooth identity and player assignments. A new game scanner must not reset controllers or shared presets.
5. **User experience:** One monitor; no dependency on an overlay surviving fullscreen. Accessible list controls must work without WebGL.
6. **No hidden requirements:** Offline, no root at runtime, no new kernel drivers, no required frontend/launcher, no Windows port.
7. **Scope transparency:** Native emulator-specific per-game profiles are not universal; acknowledge fallback behavior in both UI and documentation.

## 5. Validation and regression plan

Use synthetic/fake ROMs **only for parser/launch tests**, plus a separate set of legitimately obtained, playable ROMs for native end-to-end verification. Test isolated temporary HOME directories and a fake Flatpak launcher for automated tests; do not modify real saves in tests.

Baseline commands to preserve:

```bash
cargo test --locked
cargo clippy --all-targets -- -D warnings
node --test tests/input.test.mjs tests/ports.test.mjs
```

Extend the existing `tests/profile_flow.rs`, `tests/game_api.py`, and `tests/launch_api.py` suites to cover dynamic discovery, colliding IDs, hidden/inaccessible folders, no-follow symlink behavior, game-specific DS profiles, shared direct-launch fallback, and queued writes. Add Tauri tests after desktop conversion.

Run a physical laptop acceptance checklist with both connected Xbox One USB controllers; explicitly mark PlayStation/Switch behavior as **not hardware-tested** until devices are available. Verify single monitor/fullscreen, return-to-desktop behavior, and keyboard/mouse control for touchscreen or unsupported motion interactions.

## 6. Work boundaries and proposed PR slices

1. **PR A — melonDS diagnosis + DS discovery:** runtime scanner, stable game ID, tests, no UI framework migration.
2. **PR B — DS profile/API hardening:** safe dynamic profile authorization, on-demand generation, native melonDS shared-layout regression tests.
3. **PR C — Linux desktop shell:** Tauri packaging and UI/API migration without changing emulator profile semantics.
4. **PR D — Multi-emulator library adapters:** verified ROM formats and per-emulator capabilities.
5. **PR E — Optional known-game launch preview:** per-game suppression, single-monitor lifecycle, opt-in shortcut behavior.

Each PR should have test evidence and a short change log. Do not merge a phase with failing tests, breaking configuration migrations, or unverified direct-launch behavior.

## 7. Explicitly deferred / not planned

- Windows or macOS support.
- Universal native Linux/Steam game remapping.
- Kernel input drivers or privileged global interception.
- Injecting UI into running emulators, reading unknown internal ROM-selection events, or showing a guaranteed per-game overlay for games chosen inside unmodified emulators.
- Replacing Dolphin/melonDS/other emulator game browsers.
- ROM/BIOS/key acquisition, save-game manipulation, or cloud accounts.

## 8. Definition of done

Controller Studio is successful when a family member can **plug in a controller, open their preferred emulator from the desktop, select a game there, and play without adjusting bindings**. Controller Studio should remain the optional place to inspect or change those bindings. Any additional discovery, per-game preview, or standalone launch feature must support—not displace—that simple workflow.
