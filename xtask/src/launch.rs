//! `cargo xtask run [--release] [-- <app args>…]`: launch the desktop app.
//!
//! ```text
//! cargo xtask run [--release] [-- <app args>...]
//! ```
//!
//! `cargo run -p photocraft` does the same thing, but the package name, the profile and the
//! argument pass-through then live in whatever the developer remembers to type. This command is
//! the one place that knows them, so a file, the control server or a separate instance can be
//! started the same way every time:
//!
//! ```sh
//! cargo xtask run                                  # the debug build, with line numbers in panics
//! cargo xtask run --release                        # the release build: fast enough for real images
//! cargo xtask run --release -- image.psd           # a file to open
//! cargo xtask run -- --control 7878                # the control server, for agents
//! cargo xtask run -- --new-instance                # a second window, not the running app
//! ```
//!
//! The dev profile builds PhotoSoft's crates with `opt-level = 1` and line tables, so a panic
//! keeps its file and line; the image work is much slower than in the release build, which is why
//! `--release` is the one to use for interactive work (AGENTS.md, "Build and run").
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::path::Path;

/// The package that holds the desktop app.
pub const PACKAGE: &str = "photocraft";

/// The usage line of the command.
pub const USAGE: &str = "usage: cargo xtask run [--release] [-- <app args>...]";

/// What `cargo xtask run` was asked for.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// Build with the release profile instead of the dev profile.
    pub release: bool,
    /// The arguments for the app itself, everything after `--`.
    pub app_args: Vec<String>,
}

/// The machine preset: this checkout builds with the GNU toolchain (this machine has no MSVC
/// linker), keeps the target directory out of the path with spaces (MinGW's `dlltool` and `ar`
/// get their paths unquoted) and finds the fonts in the `craft-fonts` checkout. Every value is
/// only a default: an environment variable of the same name wins.
#[cfg(windows)]
mod preset {
    /// The GNU toolchain to build with.
    pub const TOOLCHAIN: &str = "1.95.0-x86_64-pc-windows-gnu";
    /// The target directory: MinGW's `dlltool` cannot quote a path with spaces.
    pub const TARGET_DIR: &str = r"F:\pc-target";
    /// The fonts checkout (`storytold/craft-fonts`).
    pub const FONTS_DIR: &str = r"F:\craft-fonts";
}

/// The options of `cargo xtask run`, or the usage text when `-h` was asked for.
pub fn parse(args: &[&str]) -> Result<Options, String> {
    let mut opts = Options::default();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match *arg {
            "--release" => opts.release = true,
            "--debug" => opts.release = false,
            // Everything after `--` belongs to the app, not to this command.
            "--" => {
                opts.app_args = rest.map(|a| (*a).to_string()).collect();
                return Ok(opts);
            }
            "-h" | "--help" => return Err(USAGE.into()),
            other => return Err(format!("unknown option `{other}`\n\n{USAGE}")),
        }
    }
    Ok(opts)
}

/// The `cargo` invocation that starts the app.
pub fn command(opts: &Options) -> std::process::Command {
    let mut c = crate::cargo();
    #[cfg(windows)]
    {
        // The GNU toolchain: this machine has no MSVC linker.
        if std::env::var_os("RUSTUP_TOOLCHAIN").is_none() {
            c.arg(format!("+{}", preset::TOOLCHAIN));
        }
        // The target directory cannot live under a path with spaces (MinGW's `dlltool`).
        if std::env::var_os("CARGO_TARGET_DIR").is_none() {
            c.env("CARGO_TARGET_DIR", preset::TARGET_DIR);
        }
        if std::env::var_os("CRAFT_FONTS_DIR").is_none() {
            c.env("CRAFT_FONTS_DIR", preset::FONTS_DIR);
        }
    }
    // `cargo --release run` is not a thing: the flag belongs to the subcommand.
    c.args(["run", "-p", PACKAGE]);
    if opts.release {
        c.arg("--release");
    }
    c.arg("--");
    c.args(&opts.app_args);
    c
}

pub fn run(_root: &Path, args: &[&str]) -> Result<(), String> {
    let opts = match parse(args) {
        Ok(opts) => opts,
        // `-h` is not an error: the usage text is the answer.
        Err(e) if e == USAGE => {
            println!("{USAGE}");
            return Ok(());
        }
        Err(e) => return Err(e),
    };
    let what = format!("cargo {}run -p {PACKAGE} -- {}", if opts.release { "--release " } else { "" }, opts.app_args.join(" "));
    crate::run(command(&opts), &what)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_the_dev_profile_without_app_arguments() {
        assert_eq!(parse(&[]).unwrap(), Options { release: false, app_args: Vec::new() });
    }

    #[test]
    fn release_and_debug_pick_the_profile() {
        assert!(parse(&["--release"]).unwrap().release);
        assert!(!parse(&["--debug"]).unwrap().release);
        // The last one wins.
        assert!(!parse(&["--release", "--debug"]).unwrap().release);
    }

    #[test]
    fn everything_after_the_double_dash_goes_to_the_app() {
        let opts = parse(&["--release", "--", "--control", "7878", "image.psd"]).unwrap();
        assert!(opts.release);
        assert_eq!(opts.app_args, vec!["--control", "7878", "image.psd"]);
    }

    #[test]
    fn an_app_argument_before_the_double_dash_is_an_error() {
        let err = parse(&["--control"]).unwrap_err();
        assert!(err.contains("unknown option `--control`"), "{err}");
        assert!(err.contains(USAGE), "{err}");
    }

    #[test]
    fn the_release_flag_goes_after_the_subcommand() {
        // `cargo --release run` is rejected by cargo ("unexpected argument '--release' found").
        let cmd = command(&Options { release: true, app_args: vec!["--new-instance".into()] });
        let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        let run_at = args.iter().position(|a| a == "run").expect("the run subcommand");
        assert!(args[run_at..].contains(&"--release".to_string()), "{args:?}");
        assert!(args[run_at..].contains(&"--new-instance".to_string()), "{args:?}");
    }
}
