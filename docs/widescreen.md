# Mario 64 DS widescreen check

Author: gamemasterplc. Original release video: https://www.youtube.com/watch?v=NR9DdLQLT1Y

The author links `sm64dswide.xdelta` from Dropbox and specifies NTSC-U version 1.1. Patch SHA-256: `8d2a6b24f4a40ae4bc92d13a64e1a889944fa490f379cabd5c01858b1997fe53` (1,365,949 bytes). Only patch data was downloaded; no game was downloaded and no downloaded program was run.

The installed game header reports `ASME`, revision byte 0 at offset 0x1E. Required revision is 1. The patch was not applied. No verified revision-0 alternative was established. The original game and saves remain unchanged. Do not force a patch onto another revision.

Screen layout options use melonDS 1.1's documented screen layout/sizing values:
https://github.com/melonDS-emu/melonDS/blob/1.1/src/frontend/ScreenLayout.h

A fullscreen window, a large single DS screen and patched widescreen rendering are separate features. The first two work without a ROM patch.

Update: a user-supplied USA Rev 1 dump was verified and patched with VCDIFF checksum verification enabled. Input SHA-256: `14aa4cd7b7a63a1b5dc0aa8f6f83c4a188e94b42027f0bb3c678353527d8b228`. Patched SHA-256: `a5a3c793c95087996c71f7f9a91ef17c7344ee45a815a413de3b9296ddee47a5`. Original and patched copies are separate. The game-specific melonDS screen aspects use id 1 (16:9), top-only sizing and Tab to swap screens. Controller regeneration preserves the verified aspect setting.
