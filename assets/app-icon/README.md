# PhotoSoft app icon

A **geometric "PS" monogram**: two caps built from a stem and two semicircular
annuli joined by a diagonal capsule, Paper on the app-blue field, with a small
Ink diamond below the pair. The diamond is the mark's only non-letterform, an
echo of the brow diamond of the kitsune that this icon replaces. It is an
original construction: not a derivative of, and not to be confused with, any
other product's logo.

## Palette

Exactly three colours (unchanged from the kitsune icon):

| Colour | Hex | Used for |
|---|---|---|
| Ink | `#0b0b0c` | the accent diamond |
| Paper | `#efe9dc` | the letterforms |
| App blue | `#2f7bf5` | the full-bleed field |

## Geometry

A 512-unit tile (`viewBox="0 0 512 512"`), rounded square with `rx=112`, no
border. The macOS renders pad it onto Apple's 824/1024 icon grid; the Windows
and Linux renders use the whole tile, as the other Crafting Apps do.

Construction (all coordinates are in `photocraft.svg`):

- "P": a 66-unit-wide stem (x 148–214, y 140–372) with a semicircular bowl to
  the right (outer radius 66, centred on the stem's right edge; stroke 46);
  counter radius 20.
- "S": two semicircular half-annuli (outer radius 63, stroke 46, inner radius
  17): the upper one opens to the lower right, the lower one to the upper
  left, joined by a 46-unit-wide capsule whose ends are round. The terminals
  are rounded (r 23). Spans the same y 140–372; the two arcs are centred on
  x 359 and optically centred against the P.
- Accent: an Ink diamond 38 units across, centred at (256, 436).
- The pair is centred horizontally on the tile.

## Provenance

An original geometric vector construction made for this project (hand-written
parametric SVG, no third-party material, no traced artwork). Not "Ps" artwork
borrowed from anywhere else: the letterforms were derived from primitives
(stems, semicircles, a capsule, a rotated square). Licensed like the rest of
the project, MIT OR Apache-2.0. License: see `LICENSE.txt`.

## Files

- `photocraft.svg`: canonical master (hand-written geometric SVG).
- `photocraft-small.svg`: identical master; also the hicolor scalable icon and
  what `packaging/macos/dmg/background.svg` links to.
- `photocraft-1024.png`: 1024 px render on the macOS grid.
- `photocraft.icns`: macOS bundle icon (`CFBundleIconFile`); PNG entries only
  (`ic07`–`ic14`, `ic10`). Regenerate canonically with `packaging/icons.sh` on
  macOS (it uses `iconutil`).
- `photocraft.ico`: Windows icon, 16–256 px, embedded in the `.exe` by
  `apps/photocraft/build.rs`; packed by `cargo xtask ico` from `photocraft.svg`.
- `hicolor/<size>/apps/ai.storyteller.photocraft.png` (16–512) and
  `hicolor/scalable/...svg`: Linux icon theme.

The app also sets the window icon and Wayland app ID at runtime
(`apps/photocraft/src/app_icon.rs`, `main.rs`). On Windows the taskbar shows
the icon of the Start Menu shortcut that launches the exe, so the MSI's
`<Icon Id>` keeps the `.exe` extension (ICE50); `app_icon.rs` documents the
details and tests the `.ico` sizes and the WiX icon references.

## Regenerate

Replace `photocraft.svg` (and `photocraft-small.svg`), then run
`packaging/icons.sh`. It needs `resvg`, plus `iconutil` on macOS for the
`.icns`; the `.ico` is packed by `cargo xtask ico`. The outputs are committed,
so packaging never needs these tools.
