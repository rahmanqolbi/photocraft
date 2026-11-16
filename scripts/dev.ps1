<#
.SYNOPSIS
  PhotoSoft developer helper: one place for the commands you actually type.

.DESCRIPTION
  Wraps the Rust toolchain with the fork's conventions:
    * a project-local CARGO_TARGET_DIR (the global kache wrapper can lose its
      shared cache mid-build and fail the build with "failed to write ... fingerprint");
    * the upstream task runner (`cargo xtask ...`);
    * the rebrand script, so a sync with upstream is one command.

.EXAMPLE
  .\scripts\dev.ps1 build            # build the desktop app
  .\scripts\dev.ps1 run              # build and run the desktop app
  .\scripts\dev.ps1 test -p photocraft-engine
  .\scripts\dev.ps1 snap             # offscreen UI render -> a PNG you can look at
  .\scripts\dev.ps1 sync             # fetch upstream and merge it into photosoft/base
  .\scripts\dev.ps1 rebrand -Check   # what the rebrand script would change
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet('build', 'run', 'cli', 'test', 'clippy', 'snap', 'xtask', 'sync', 'rebrand', 'clean', 'help')]
    [string]$Task = 'help',

    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Rest,

    # Where Cargo writes its build output. Defaults to H: when the Rust caches live
    # there (see docs/photosoft.md > Disk layout); otherwise the project directory.
    [string]$TargetDir
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    # Where Cargo writes its build output.
    #
    # This machine's Cargo (a fork of cargo 1.101) ignores CARGO_TARGET_DIR: it puts
    # every project in ~/.cargo/targets/<project>-<hash> and compiles the dependencies
    # once into ~/.cargo/targets/shared-deps. The build tree is therefore moved with a
    # directory junction, not an environment variable (see docs/photosoft.md).
    # -TargetDir still works for a throwaway lane, because cargo's --target-dir is
    # honoured for the crates it compiles itself.
    if ($TargetDir) {
        $env:CARGO_TARGET_DIR = $TargetDir
        Write-Host "target dir: $TargetDir" -ForegroundColor DarkGray
    }
    $env:CARGO_TERM_COLOR = 'never'

    switch ($Task) {
        'build' { cargo build -p photocraft @Rest }
        'run'   { cargo run -p photocraft @Rest }
        'cli'   { cargo run -p photocraft-cli -- @Rest }
        'test'  { cargo test @Rest }
        'clippy' { cargo clippy --workspace --all-targets -- -D warnings @Rest }
        'snap'  {
            # Offscreen UI render: no window, no focus stealing. Look at the PNG.
            cargo run -p photocraft-ui-egui --example snapshot @Rest
        }
        'xtask' { cargo xtask @Rest }
        'sync' {
            git fetch upstream
            git checkout photosoft/base
            git merge upstream/main
            python scripts/rebrand.py
            Write-Host 'Rebrand re-applied. Review with: git diff' -ForegroundColor Yellow
        }
        'rebrand' {
            if ($Rest -contains '-Check') { python scripts/rebrand.py --check }
            else { python scripts/rebrand.py }
        }
        'clean' { cargo clean @Rest }
        default {
            Get-Help $PSCommandPath -Detailed
        }
    }
}
finally {
    Pop-Location
}
