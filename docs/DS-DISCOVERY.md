# DS discovery and exact identification

The 0.4.0 source adds discovery in `~/Games/Nintendo/DS` at startup and on
**Refresh DS library**. It accepts standard DS `.nds` files, including subfolders,
spaces and Unicode filenames. It checks executable/table/banner ranges and
regular-file type, rather than trusting extensions or requiring a known game code.
Trimmed ROMs do not have to match their cartridge-capacity byte. These checks do
not prove that a game is playable.

Each record uses the entire ROM's SHA-256. Renaming keeps its control identity;
changed bytes get another identity. Same titles get a short hash suffix. Missing
records retain preferences. Identical content at multiple locations is reported
and cannot launch until the duplicate is resolved, because the locations may
have different ROM-adjacent saves. Studio never moves or renames saves. Renaming
a ROM can still change the save filename the emulator expects.

Scanning walks directory descriptors without following links, refuses special
files, limits depth to 12, visits at most 4096 entries, retains at most 512 ROM
identities, and stops scanning after an eight-second budget (an in-progress digest
can add up to two seconds). A ROM is at most 512 MiB. Digest subprocesses have a
two-second timeout. Files changed during scanning are rejected. Refresh can report
incomplete results; limits do not silently delete existing identities. Unchanged
files cache their digest using device/inode/size and nanosecond change/modification
times. Launch rechecks the header and content hash rather than trusting that cache.
The registered index is bounded and validated before authorizing an exact per-game
config path. No wildcard path authorization is added.

New isolated DS configurations are generated on request, rather than for every
file in the library. Legacy ASME/CLJE preferences are copied once to the recognized
content identity; restoring shared inheritance is not undone on the next refresh.
The old entries/configs remain compatible. The verified widescreen copy keeps its
existing aspect settings. Keyboard/touch input and shared direct-use controls remain.

## Why melonDS needs an integration

Stock melonDS 1.1 publishes a version/FPS window title, without a loaded ROM ID.
A scanner, launch command line, or recent-file list cannot reliably identify a ROM
selected later inside that emulator. Studio does not guess from those signals.

`integrations/melonds-1.1-controller-id.patch` is a small source patch for official
melonDS tag **1.1**, commit `b86390e4428bf38ce4c1ce0e9ca446d6d25955e8`.
It hashes the bytes loaded by melonDS and appends a versioned public title marker:

```
[CS-ROM-v1:<64 lowercase SHA-256 characters>]
```

It publishes only after a successful cartridge load, clears on failed loads/eject,
and keeps the marker on paused titles. Titles do not contain ROM paths or save
contents. Switching games changes the marker. Studio matches only an exact,
available library identity; unknown hashes and multiple different DS sessions
remain unidentified. Selecting a game in the UI does not claim to activate its
isolated config. Direct emulator use still uses shared controls.

The patch is separate from the MIT helper and modifies GPLv3-or-later melonDS
sources. Retain upstream's license and provide corresponding patched sources if
distributing a built emulator. Source: https://github.com/melonDS-emu/melonDS/tree/1.1

Prepare a separate clean source checkout:

```sh
git clone --branch 1.1 --depth 1 https://github.com/melonDS-emu/melonDS.git melonDS-1.1
python3 scripts/prepare-melonds-identity.py /absolute/path/to/melonDS-1.1
```

The preparation tool checks exact before/after SHA-256 values and stages all
outputs before writing. Wrong versions and repeat application stop without changes.
It does not download, build, install, replace a Flatpak, or modify saves.
Build using upstream's instructions **on the target Mint/compatible SDK**, then
verify the integration before changing the family's launcher. A binary built
against Debian 13 must not be assumed compatible with Mint 22.3.

## Verification and remaining native gate

Rust parser/profile/storage tests and the disposable HTTP test use synthetic ROMs.
They validate discovery, identities, failures, and exact launch arguments; they
are not gameplay results. The fake Flatpak integration never runs a real game.

The user has only the two existing DS games. Those two were tested on a private
Debian Xvfb display: exact loaded hashes, switching from the emulator's own Open
ROM dialog, Studio following both identities, and clean emulator exit all passed.
This is real-ROM integration evidence, not native Mint gameplay acceptance.

The target Mint gate still requires playable user-supplied DS ROMs,
the patched emulator, native game switching/eject/pause/exit, shared controller and
keyboard play independently of Studio, correct per-game launch/save paths, and
unchanged Dolphin/player behavior. No later manifest phase is enabled by synthetic
or build-only results.
