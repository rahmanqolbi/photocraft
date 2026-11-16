#!/usr/bin/env python3
"""Re-brand the PhotoCraft fork into an independent app (PhotoSoft).

The upstream project (storytold/photocraft) is MIT OR Apache-2.0, so the code may be
forked, modified and redistributed. What the licence does *not* cover is the ArtCraft
brand: the name, the wordmark and the logos are trademarks (docs/brand/LICENSE-brand.txt).
This script therefore rewrites the *visible* identity and leaves the *machine* names alone:

  visible identity (rewritten)      machine identity (kept)
  ---------------------------------  ---------------------------------------------
  app id  com.photosoft.photosoft  crate names      photocraft-doc, photocraft-io, ...
  name    PhotoCraft                 binary names     photocraft.exe, photocraft-cli
  icon    assets/app-icon/*           env vars         PHOTOCRAFT_CONTROL_PORT, ...
  package metadata                   file formats     .pcraft, x-photocraft
  docs/user/*                        command ids      layer.newAdjustmentLayer.curves

Why keep the machine names? They are internal identifiers that hundreds of crates,
scripts and tests depend on. Renaming them buys nothing visible and costs a 29-crate
rename plus a permanent conflict tax on every upstream merge.

Run it after every upstream sync:

    python scripts/rebrand.py            # apply
    python scripts/rebrand.py --check    # dry run, prints what would change
    python scripts/rebrand.py --with-docs  # also rewrite the internal docs' prose

The script is idempotent: running it twice changes nothing the second time.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

OLD_APP_ID = "ai.storyteller.photocraft"
NEW_APP_ID = "com.photosoft.photosoft"
OLD_NAME = "PhotoCraft"
NEW_NAME = "PhotoSoft"
OLD_VENDOR = "Learning Machines LLC"
NEW_VENDOR = "PhotoSoft"
OLD_HOME = "https://getartcraft.com/apps/photocraft"
NEW_HOME = "https://github.com/rahmanqolbi/photocraft"

# Files that carry the product's visible identity. Everything else keeps the upstream
# wording on purpose: the engineering docs describe upstream's engine, and rewriting
# them would bury every future merge in noise.
CORE_GLOBS = [
    "crates/**/*.rs",
    "crates/**/*.toml",
    "crates/**/*.md",
    "crates/**/*.wgsl",
    "crates/ui-egui/src/i18n/*.tsv",
    "apps/**/*.rs",
    "apps/**/*.toml",
    "apps/**/*.md",
    "apps/**/*.html",
    "packaging/**/*",
    "assets/app-icon/README.md",
    "assets/app-icon/LICENSE.txt",
    "xtask/**/*.rs",
    "xtask/**/*.toml",
    # README.md is the fork's own document: it says PhotoSoft is a fork of PhotoCraft
    # by design. Excluded; maintained by hand.
    # NOTICE is an attribution file, like ATTRIBUTION.md: the upstream attribution lines
    # (modified version of PhotoCraft, upstream copyright) must stay PhotoCraft, and the
    # blanket rewrite would bury that. Excluded; it is maintained by hand.
    "docs/user/**/*",
    "docs/releasing.md",
    "docs/mcp.md",
    "docs/control-protocol.md",
    "docs/printing.md",
    "docs/plugins.md",
    "book/**/*",
    ".github/workflows/*.yml",
]

# The app id is a machine identifier, but it is *ours* now: it has to be rewritten
# everywhere, including the internal docs that describe packaging.
APPID_GLOBS = ["**/*"]

# Files renamed when the app id changes.
RENAMES = [
    ("assets/app-icon/hicolor/{s}x{s}/apps/{old}.png", "assets/app-icon/hicolor/{s}x{s}/apps/{new}.png"),
    ("assets/app-icon/hicolor/scalable/apps/{old}.svg", "assets/app-icon/hicolor/scalable/apps/{new}.svg"),
    ("packaging/linux/{old}.desktop", "packaging/linux/{new}.desktop"),
    ("packaging/linux/{old}.metainfo.xml.in", "packaging/linux/{new}.metainfo.xml.in"),
    ("packaging/linux/{old}.mime.xml", "packaging/linux/{new}.mime.xml"),
    ("packaging/linux/flatpak/{old}.yml", "packaging/linux/flatpak/{new}.yml"),
    ("packaging/linux/flatpak/{old}.bundle.yml", "packaging/linux/flatpak/{new}.bundle.yml"),
]
SIZES = [16, 24, 32, 48, 64, 128, 256, 512]

# Files the rebrand must not touch.
#
# scripts/rebrand.py: the script holds the *old* names as its constants. Rewriting
# itself would turn them into the new names and the script would stop detecting
# anything (it becomes a no-op that reports "already renamed").
#
# docs/photosoft.md: it documents the old -> new mapping, so it must keep the old
# names to stay readable.
#
# xtask/src/scorecard.rs generates docs/scorecard.md, which is committed and checked in
# CI (`cargo xtask scorecard --check` fails when the generator and the file disagree).
# Upstream's scorecard is upstream's bookkeeping: this fork keeps both the generator and
# the file as they are, so they stay in sync.
EXCLUDE = {"xtask/src/scorecard.rs", "scripts/rebrand.py", "docs/photosoft.md"}

TEXT_SUBS = [
    (OLD_APP_ID, NEW_APP_ID),
    (re.compile(r"ai\.storyteller(?!\.\w)"), "com.photosoft"),
    (OLD_NAME, NEW_NAME),
    (OLD_VENDOR, NEW_VENDOR),
    (OLD_HOME, NEW_HOME),
    # The issue tracker and the releases are the fork's, not upstream's.
    ("github.com/storytold/photocraft/issues", "github.com/rahmanqolbi/photocraft/issues"),
    ("github.com/storytold/photocraft/releases", "github.com/rahmanqolbi/photocraft/releases"),
]

# Rules that only apply to a subtree. The packaging scripts install the app under a
# suite folder named after the upstream brand ("ArtCraft"), and point the help link
# at upstream's Discord; both have to become ours.
SCOPED_SUBS = {
    "packaging/": [
        (re.compile(r"ArtCraft"), "PhotoSoft"),
        (re.compile(r"discord\.gg/artcraft"), "github.com/rahmanqolbi/photocraft"),
        (re.compile(r"github\.com/storytold/photocraft"), "github.com/rahmanqolbi/photocraft"),
        # The demo screenshot and the release feed come from the fork's own repository.
        (re.compile(r"storytold/photocraft(?![-\w])"), "rahmanqolbi/photocraft"),
    ],
}

BINARY_SUFFIXES = {".png", ".ico", ".icns", ".ttf", ".otf", ".woff", ".woff2", ".bin", ".gz", ".jpg", ".jpeg", ".webp", ".tiff", ".pdf"}


def tracked_files(root: Path) -> list[Path]:
    out = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"],
        check=True,
        capture_output=True,
    ).stdout
    return [root / p.decode("utf-8") for p in out.split(b"\0") if p]


def glob_to_regex(pattern: str) -> re.Pattern[str]:
    """Translate a git-style glob into a regex.

    `PurePath.match` does not treat `**` as "any number of directories" in the middle of
    a pattern, so `crates/**/*.rs` silently matches nothing. Doing it by hand keeps the
    patterns in this file behaving the way they read.
    """
    out: list[str] = []
    i = 0
    while i < len(pattern):
        if pattern.startswith("**/", i):
            out.append("(?:[^/]+/)*")
            i += 3
        elif pattern.startswith("**", i):
            out.append(".*")
            i += 2
        elif pattern[i] == "*":
            out.append("[^/]*")
            i += 1
        elif pattern[i] == "?":
            out.append("[^/]")
            i += 1
        else:
            out.append(re.escape(pattern[i]))
            i += 1
    return re.compile("^" + "".join(out) + "$")


def matches(path: Path, root: Path, globs: list[str]) -> bool:
    rel = path.relative_to(root).as_posix()
    return any(glob_to_regex(g).match(rel) for g in globs)


def substitute(text: str, rel: str = "") -> str:
    for pattern, repl in TEXT_SUBS:
        text = pattern.sub(repl, text) if hasattr(pattern, "sub") else text.replace(pattern, repl)
    for prefix, rules in SCOPED_SUBS.items():
        if rel.startswith(prefix):
            for pattern, repl in rules:
                text = pattern.sub(repl, text)
    return text


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check", action="store_true", help="dry run: print what would change")
    ap.add_argument("--with-docs", action="store_true", help="also rewrite the internal docs' prose")
    args = ap.parse_args()

    root = Path(__file__).resolve().parent.parent
    files = tracked_files(root)
    changed: list[str] = []
    renamed: list[str] = []

    # 1. Content.
    for path in files:
        if not path.is_file() or path.suffix.lower() in BINARY_SUFFIXES:
            continue
        rel = path.relative_to(root).as_posix()
        if rel in EXCLUDE:
            continue
        in_core = matches(path, root, CORE_GLOBS)
        if not (in_core or matches(path, root, APPID_GLOBS)):
            continue
        try:
            original = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        updated = substitute(original, rel)
        if not in_core and not args.with_docs:
            # Outside the core scope only the app id may change.
            updated = original.replace(OLD_APP_ID, NEW_APP_ID)
            updated = re.sub(r"ai\.storyteller(?!\.\w)", "com.photosoft", updated)
        if updated != original:
            changed.append(rel)
            if not args.check:
                path.write_text(updated, encoding="utf-8", newline="")

    # 2. Renames.
    for old_tpl, new_tpl in RENAMES:
        for size in SIZES if "{s}" in old_tpl else [None]:
            old = old_tpl.format(s=size or "", old=OLD_APP_ID, new=NEW_APP_ID)
            new = new_tpl.format(s=size or "", old=OLD_APP_ID, new=NEW_APP_ID)
            src = root / old
            if not src.exists():
                continue
            renamed.append(f"{old} -> {new}")
            if not args.check:
                dst = root / new
                dst.parent.mkdir(parents=True, exist_ok=True)
                src.rename(dst)

    print(f"content: {len(changed)} file(s)")
    for rel in changed:
        print(f"  ~ {rel}")
    print(f"renamed: {len(renamed)} file(s)")
    for rel in renamed:
        print(f"  > {rel}")
    if args.check:
        print("\n(dry run: nothing written)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
