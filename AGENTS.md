# AGENTS.md: guide for AI agents and contributors working on **PhotoSoft**

**PhotoSoft is a fork of [PhotoCraft](https://github.com/storytold/photocraft)** (upstream:
`storytold/photocraft`), rebranded to be an independent app. This file is the fork's rulebook:
it overrides upstream's `AGENTS.md` wherever the two disagree. For the engine's design
rationale read upstream's docs (`docs/architecture.md`, `docs/development.md`,
`docs/contributing.md`, `docs/ui-design.md`) — they still describe this code accurately.

## 0. Orientation (5 minutes)

| Read | Why |
|---|---|
| `docs/photosoft.md` | **This fork's dev guide**: branch model, the dev loop, the rebrand script, the licence checklist |
| `docs/architecture.md` | Crate map, dependency layers, the engine/UI seam, document model |
| `docs/development.md` | Build, test, run, drive the app programmatically, debug tricks |
| `docs/contributing.md` | The "add a command" checklist (upstream's rules still apply) |
| `docs/control-protocol.md` | JSON control channel: how agents drive and screenshot the running app |
| `docs/ui-design.md` | Design tokens, themes, widgets |

## 1. What this fork is (and is not)

- **Code**: MIT OR Apache-2.0, from upstream. Forked, modified, redistributable. Keep
  `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE` and `ATTRIBUTION.md` intact; `NOTICE` states the
  modifications (Apache-2.0 §4) and credits upstream's contributors unchanged.
- **Brand**: the ArtCraft name, wordmark and logos are **trademarks, not open source**. This
  fork does not use them; `docs/brand/` was removed. Do not reintroduce them, do not ship
  `getartcraft.com` links, do not imply endorsement by the ArtCraft Team.
- **Identity**: the visible name is `PhotoSoft`, the app id is `com.photosoft.photosoft`.
- **Machine names stay**: crates (`photocraft-*`), binaries (`photocraft.exe`), env vars
  (`PHOTOCRAFT_*`), command ids, the `.pcraft` format. They are internal identifiers; renaming
  them buys nothing visible and costs a 29-crate rename plus a permanent conflict tax on every
  upstream merge. Do not rename them casually.

## 2. Branch model

```
origin/main        mirror of upstream (fast-forward only, never commit here)
upstream/main      upstream's tip
photosoft/base     upstream + the rebrand  ← work here
photosoft/feat-x   a feature branch, branched off photosoft/base
```

```sh
git remote add upstream https://github.com/storytold/photocraft.git   # once
git fetch upstream
git checkout photosoft/base && git merge upstream/main   # merge, never rebase
python scripts/rebrand.py                                     # re-apply the identity
git diff                                                      # review
```

**Merge, don't rebase**: upstream moves daily, and a rebase of a large fork rewrites history
for no benefit. After a merge, the rebrand script fixes the identity again (it is idempotent).

Never let a rebrand-era conflict grow: the rebrand touches ~100 files of prose and metadata.
If a merge leaves many conflicts, abort the merge and re-run the rebrand script on a clean
`upstream/main` instead of resolving them by hand.

## 3. Adding a feature (upstream's seams still apply)

1. **Algorithm** goes in the lowest crate that fits (`algo`, `paint`, `vector`, `text`, `cms`),
   depth-agnostic (8/16/32-bit), per-tile, deterministic.
2. **Command** goes in a new module `crates/engine/src/cmd/<area>_cmds.rs` (do not grow a shared
   file), registered in `crates/engine/src/cmd.rs`. Give it an id from
   `crates/ui-egui/src/menu_catalog.rs` so the menu item goes live for free; a feature that is
   not a Photoshop item uses a descriptive id (`photosoft.batch.watermark`) and an empty menu path.
3. **UI**: thin and data-driven. State in `crates/ui-egui/src/state.rs`, colours from
   `theme::Tokens`. Schema-driven dialogs (`ui-egui/src/dialogs.rs`, `filter_dialog.rs`) instead of
   hand-drawn ones.
4. **Tests** for every change. Commands never panic on bad input (return `Err`).

## 4. Rules that still come from upstream

- **Never crash.** No `unwrap`/`expect`/`panic!`/`unreachable!`/`todo!` in non-test code, no
  `unsafe` outside `crates/tablet`. Errors go through `Result` and `?`.
- **Everything is a command.** The UI, the CLI, the control channel and the MCP server all
  dispatch the same command ids.
- **Layering** is enforced: `cargo xtask layers`. Nothing below `ui-egui` may use egui, eframe,
  winit or rfd. A new crate is registered in `xtask/src/layers.rs`.
- **No format or colour assumptions.** Bit depth and colour model are runtime data; conversions
  go through `photocraft-cms`.
- **Clean-room.** Implement from public specs and observation; never copy proprietary code,
  shaders, profiles or assets. Third-party assets are permissively licensed, keep their licence
  next to them and get a row in `ATTRIBUTION.md`.
- **Never commit font files.** Fonts come from `CRAFT_FONTS_DIR` (a `craft-fonts` checkout).
- **Never break wasm**: `cargo xtask wasm`.

## 5. What this fork does *not* maintain

Upstream's parity bookkeeping is upstream's job. Skip `cargo xtask parity`,
`docs/parity-checklist.md`, `docs/gaps.md`, `docs/scorecard.md` and the scorecard/perf
baselines unless a change moves a number you actually track. Do not edit them to look busy.

Keep green: `cargo test -p <crate>`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo xtask layers`.

## 6. Before you finish

```sh
cargo test -p <crates you touched>
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask layers
python scripts/rebrand.py --check     # the identity must be consistent
git status                            # no stray target/, no upstream's brand assets
```

Then commit: small, focused, a clear subject. Push to `origin`, open a PR against
`photosoft/base` (not `main`).

## 7. Where things are tracked

- `docs/photosoft.md` — this fork's dev guide (branch model, dev loop, rebrand, licence).
- `scripts/rebrand.py` — the identity rewrite; `scripts/dev.ps1` — the command shortcuts.
- `docs/development.md`, `docs/contributing.md` — upstream's build/test/contribution rules.
- `docs/roadmap.md`, `docs/gaps.md`, `docs/target-app-parity.md` — upstream's parity tracking
  (read-only for this fork unless a change moves a number).
- `plan/`, `log/` — local, gitignored: this fork's own research and dev log.

## 8. Contributor credits

`contributors/people.toml` and `contributors/contributors.json` are upstream's: keep every
upstream credit unchanged. Add your own entry only if you want to be named; never edit or guess
anyone else's entry.
