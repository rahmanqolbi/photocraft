# PhotoSoft — the fork's dev guide

PhotoSoft is a fork of [PhotoCraft](https://github.com/storytold/photocraft) (MIT OR
Apache-2.0). This file is the practical guide for working on it: how the branches are laid out,
how to build and run it without pain, how the rebrand is kept consistent, and what the licence
asks of you. The engine's design is documented upstream (`docs/architecture.md`,
`docs/development.md`, `docs/contributing.md`); the rules for agents and contributors are in
[`AGENTS.md`](../AGENTS.md).

## TL;DR

```powershell
.\scripts\dev.ps1 build             # build the desktop app
.\scripts\dev.ps1 run               # build and run it
.\scripts\dev.ps1 test -p photocraft-engine
.\scripts\dev.ps1 clippy
.\scripts\dev.ps1 snap              # offscreen UI render -> a PNG to look at
.\scripts\dev.ps1 xtask layers      # upstream's task runner
.\scripts\dev.ps1 sync              # fetch upstream, merge it, re-apply the identity
.\scripts\dev.ps1 rebrand -Check    # what the identity script would change
```

Without the wrapper, every command is plain Cargo:

```sh
cargo build -p photocraft
cargo run -p photocraft
cargo run -p photocraft-ui-egui --example snapshot
cargo test -p photocraft-engine --lib
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask layers
```

## Requirements

| | |
|---|---|
| Rust | 1.95 or newer (the workspace is edition 2024) |
| Python | 3.x, for `scripts/rebrand.py` only |
| Optional | a `craft-fonts` checkout + `CRAFT_FONTS_DIR` for the bundled CJK fonts |
| Optional | `cargo-nextest` for faster test loops (`cargo nextest run`) |

## Read this once: the `CARGO_TARGET_DIR` trap

Your global `~/.cargo/config.toml` sets `rustc-wrapper = "kache"`. kache keeps one **shared**
target directory (`~/.cargo/targets/shared-deps`) for every project on the machine, and its
garbage collector (`build.gc.frequency = "always"`, `max-unused = "12 hours"`) may delete that
directory **while a build is running**. The build then dies with:

```text
error: failed to write `.../shared-deps/debug/build/<crate>/<hash>/fingerprint/invoked.timestamp`
Caused by: The system cannot find the path specified. (os error 3)
```

That is not a code error and not a broken checkout — it is the cache being vacuumed underneath
the build. **Keep the target directory on a drive with room, and outside the shared cache:**

```powershell
$env:CARGO_TARGET_DIR = "H:\cargo\targets\photosoft"
```

`scripts/dev.ps1` sets this for you (H: when it exists, otherwise the project directory) and
prints the path it picked. A full debug build with the test binaries is roughly 10 GB.

### Disk layout on this machine

| Path | What | Size |
|---|---|---|
| `H:\cargo\targets\photosoft` | this project's build output (`CARGO_TARGET_DIR`) | ~10 GB |
| `H:\cargo\targets\<project>-<hash>` | the other Rust projects, reached through a **directory junction** at `C:\Users\Qolbi\.cargo\targets\` | ~9 GB |
| `C:\Users\Qolbi\.cargo\registry` | the crate registry | 2.8 GB |
| `C:\Users\Qolbi\.cargo\targets\shared-deps` | the shared dependency cache | 12.8 GB |

The junctions are why the projects keep working from their usual paths while the bytes live
on H:. To move another one:

```powershell
robocopy "C:\Users\Qolbi\.cargo\targets\<dir>" "H:\cargo\targets\<dir>" /MOVE /E
cmd /c "mklink /J C:\Users\Qolbi\.cargo\targets\<dir> H:\cargo\targets\<dir>"
```

Do it while no build is running: Cargo holds a lock file in the target directory, and a
half-moved directory is worse than an unmoved one.

Two builds in the same target directory block each other on Cargo's build lock. If you need two
at once (a background build while you iterate), give the second one its own directory:

```powershell
$env:CARGO_TARGET_DIR = "H:\cargo\targets\photosoft-agent2"
```

## Branch model

```
origin/main        mirror of upstream (fast-forward only — never commit here)
upstream/main      upstream's tip
photosoft/base     upstream + the rebrand          ← the branch you work on
photosoft/feat-x   a feature branch off photosoft/base
```

`origin/main` is kept byte-identical to `upstream/main`. That is what makes
`git diff upstream/main` show exactly what this fork changed, and it keeps the option of
dropping the fork's changes without touching upstream's history.

### Syncing with upstream

```powershell
.\scripts\dev.ps1 sync
```

which is:

```sh
git fetch upstream
git checkout photosoft/base
git merge upstream/main      # merge, never rebase
python scripts/rebrand.py    # re-apply the identity
git diff                     # review what the merge changed
```

**Merge, don't rebase.** Upstream pushes daily; rebasing rewrites your history for no benefit
and turns every upstream commit into a conflict to replay. A merge keeps upstream's lineage
visible in the log and makes conflicts one-off.

**Do not replay the old rebrand patch.** The first attempt at this fork (branch
`personal/photosoft-v2`) carried the identity as a 391-file patch. Upstream moved underneath
it, and replaying it produced 170+ conflicted files. The identity is now a **script**
(`scripts/rebrand.py`) that rewrites the current tree, so a sync is a merge plus one command.

If a merge leaves a pile of conflicts, abort it and start again:

```sh
git merge --abort
git checkout upstream/main && git branch -f photosoft/base
git checkout photosoft/base && python scripts/rebrand.py
```

## The rebrand script

`scripts/rebrand.py` rewrites the **visible** identity and deliberately leaves the **machine**
identity alone:

| Visible identity (rewritten) | Machine identity (kept) |
|---|---|
| product name `PhotoCraft` → `PhotoSoft` | crate names `photocraft-doc`, `photocraft-io`, … |
| app id `ai.storyteller.photocraft` → `com.photosoft.photosoft` | binaries `photocraft.exe`, `photocraft-cli` |
| icon (`assets/app-icon/`, an original geometric construction) | env vars `PHOTOCRAFT_CONTROL_PORT`, … |
| packaging metadata (desktop file, flatpak, MSI, DMG, deb/rpm) | command ids `layer.newAdjustmentLayer.curves` |
| the 26 UI catalogues (`crates/ui-egui/src/i18n/*.tsv`) | the `.pcraft` format and `x-photocraft` |

Why keep the machine names? They are internal identifiers that hundreds of crates, scripts and
tests depend on. Renaming them buys nothing a user can see, and costs a 29-crate rename plus a
permanent conflict tax on every upstream merge.

```powershell
python scripts/rebrand.py            # apply
python scripts/rebrand.py --check    # dry run: list what would change
python scripts/rebrand.py --with-docs # also rewrite the internal docs' prose
```

The script is idempotent and reads the tracked file list from Git, so it never touches
`target/` or your own notes.

### What still says "PhotoCraft" on purpose

- The **internal engineering docs** (`docs/architecture.md`, `docs/gaps.md`, …) describe
  upstream's engine; rewriting their prose would bury every future merge in noise. Use
  `--with-docs` when you want them rewritten too.
- The **crate, binary, env-var and format names** (see the table above).
- The **upstream attribution** in `NOTICE`, `LICENSE-*`, `ATTRIBUTION.md` and
  `contributors/`: those are legal statements about upstream's work and must keep their
  original wording.

## The dev loop

### Build and run

```sh
cargo build -p photocraft        # target/debug/photocraft.exe
cargo run -p photocraft          # build + run
cargo run -p photocraft -- --help
```

### Test

```sh
cargo test -p photocraft-engine --lib        # a crate's unit tests
cargo test -p photocraft-engine --test panic_hunt -- --ignored   # command fuzzing
cargo nextest run                             # if nextest is installed
```

### Look at the UI without a window

```sh
cargo run -p photocraft-ui-egui --example snapshot
```

renders the UI offscreen to a PNG. This is the fastest way to check a visual change, and it
never steals focus.

### Iterate headlessly (the fastest way to build a feature)

The engine is driven by commands with JSON parameters, so most work needs no mouse:

```sh
cargo run -p photocraft-cli -- convert in.png out.jpg --quality 90
cargo run -p photocraft-cli -- run file.psd \
    --cmd filter.sharpen.smartSharpen --params '{"amount":80}' \
    --cmd layer.newAdjustmentLayer.curves --params '{"points":[[0,0],[64,48],[192,212],[255,255]]}' \
    --out final.png
cargo run -p photocraft-cli -- batch --actions grade.json --in ./raw --out ./graded
```

The same commands are reachable from the running app over its control channel
(`cargo run -p photocraft -- --control 8730`), which is also how you take screenshots of the
live app (`docs/control-protocol.md`).

### Upstream's task runner

```sh
cargo xtask layers          # check the dependency layering
cargo xtask wasm            # check the WebAssembly build
cargo xtask i18n-coverage   # which UI strings are untranslated
cargo xtask corpus --all    # fetch the real-file test corpora
cargo xtask test-corpus     # run the corpus tests
cargo xtask stats           # tests and lines per crate
```

## Adding a feature

Upstream's seams still apply (see `AGENTS.md` §3):

1. the algorithm goes into the lowest crate that fits (`algo`, `paint`, `vector`, `text`);
2. the command goes into a **new** module `crates/engine/src/cmd/<area>_cmds.rs`;
3. the UI is thin and schema-driven.

A command with an id from `crates/ui-egui/src/menu_catalog.rs` appears in the menu, gets a
shortcut, undo/redo, CLI, MCP and control-channel support for free. A feature that is not a
Photoshop item uses a descriptive id such as `photosoft.batch.watermark`.

## Licence checklist before you publish

- [ ] `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE` and `ATTRIBUTION.md` are present and unchanged
      in their legal statements.
- [ ] `NOTICE` states the modifications (Apache-2.0 §4 requires it) and credits upstream's
      contributors.
- [ ] The ArtCraft marks are nowhere in the build, the README or the packaging.
- [ ] Every bundled third-party asset keeps its licence file next to it and its row in
      `ATTRIBUTION.md` (Inter, JetBrains Mono, Lucide, SCOWL, libfacedetection, emoji-icon-font,
      and the craft-fonts fonts when `CRAFT_FONTS_DIR` is used).
- [ ] No font files are committed to the repository.
- [ ] `python scripts/rebrand.py --check` reports nothing to change.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `failed to write ... fingerprint/invoked.timestamp` | kache's shared cache was deleted mid-build | set `CARGO_TARGET_DIR` inside the project |
| `Blocking waiting for file lock on build directory` | two builds share one target dir | give the second build its own `CARGO_TARGET_DIR` |
| `the lock .cargo-lock is held` | a previous cargo died | delete the stale lock in the target dir |
| 170+ conflicts after a merge | an old rebrand patch is being replayed | `git merge --abort`, re-run `scripts/rebrand.py` |
| `App id ... mismatch` in a packaging test | a renamed file was not renamed with it | run `python scripts/rebrand.py` |
