//! This crate provides a programmatical way to invoke `flatc` command (e.g. from `build.rs`) to
//! generate Rust (or, in fact, any other language) helpers to work with FlatBuffers.
//!
//! NOTE: You will still need
//! [`flatc` utility](https://google.github.io/flatbuffers/flatbuffers_guide_using_schema_compiler.html)
//! version [1.10.0+](https://github.com/google/flatbuffers/releases/tag/v1.10.0) installed (there
//! are [windows binary releases](https://github.com/google/flatbuffers/releases), `flatbuffers`
//! packages for [conda](https://anaconda.org/conda-forge/flatbuffers) [Windows, Linux, MacOS],
//! [Arch Linux](https://www.archlinux.org/packages/community/x86_64/flatbuffers/)).
//!
//! # Examples
//!
//! ## Minimal useful example
//!
//! Let's assume you have `input.fbs` specification file in `flatbuffers` folder, and you want to
//! generate Rust helpers into `flatbuffers-helpers-for-rust` folder:
//!
//! ```
//! use std::path::Path;
//!
//! use flatc_rust;
//!
//! # fn try_main() -> flatc_rust::Result<()> {
//! #
//! flatc_rust::run(flatc_rust::Args {
//!     lang: "rust",  // `rust` is the default, but let's be explicit
//!     inputs: &[Path::new("./flatbuffers/input.fbs")],
//!     out_dir: Path::new("./flatbuffers-helpers-for-rust/"),
//!     ..Default::default()
//! })?;
//! #
//! #     Ok(())
//! # }
//! # try_main().ok();
//! ```
//!
//! ## Build scripts (`build.rs`) integration example
//!
//! It is common to have FlatBuffers specifications as a single source of truth, and thus, it is
//! wise to build up-to-date helpers when you build your project. There is a built-in support for
//! [build scripts in Cargo], so you don't need to sacrifice the usual workflow (`cargo build /
//! cargo run`) in order to generate the helpers.
//!
//! 1. Create `build.rs` in the root of your project (along side with `Cargo.toml`) or follow the
//!    official documentation about build scripts.
//! 2. Adapt the following example to fit your needs and put it into `build.rs`:
//!
//!     ```no_run
//!     extern crate flatc_rust;  // or just `use flatc_rust;` with Rust 2018 edition.
//!
//!     use std::path::Path;
//!
//!     fn main() {
//!         println!("cargo:rerun-if-changed=src/message.fbs");
//!         flatc_rust::run(flatc_rust::Args {
//!             inputs: &[Path::new("src/message.fbs")],
//!             out_dir: Path::new("target/flatbuffers/"),
//!             ..Default::default()
//!         }).expect("flatc");
//!     }
//!     ```
//! 3. Add `flatc-rust` into `[build-dependencies]` section in `Cargo.toml`:
//!
//!     ```toml
//!     [build-dependencies]
//!     flatc-rust = "*"
//!     ```
//! 4. Add `flatbuffers` into `[dependencies]` section in `Cargo.toml`:
//!
//!     ```toml
//!     [dependencies]
//!     flatbuffers = "0.5"
//!     ```
//! 5. Include the generated helpers in your `main.rs` or `lib.rs`:
//!
//!     ```ignore
//!     #[allow(non_snake_case)]
//!     #[path = "../target/flatbuffers/message_generated.rs"]
//!     pub mod message_flatbuffers;
//!     ```
//! 5. Use the helpers like any regular Rust module ([example projects])
//!
//! [build scripts in Cargo]: https://doc.rust-lang.org/cargo/reference/build-scripts.html
//! [example projects]: https://github.com/frol/flatc-rust/tree/master/examples

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process;

use log::info;

pub type Error = io::Error;
pub type Result<T> = io::Result<T>;

fn err_other<E>(error: E) -> Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    Error::new(io::ErrorKind::Other, error)
}

/// This structure represents the arguments passed to `flatc`
///
/// # Example
///
/// ```
/// use std::path::Path;
///
/// let flatc_args = flatc_rust::Args {
///     lang: "rust",
///     inputs: &[Path::new("./src/input.fbs")],
///     out_dir: Path::new("./flatbuffers-helpers-for-rust/"),
///     ..Default::default()
/// };
/// ```
#[derive(Debug)]
pub struct Args<'a> {
    /// Specify the programming language (`rust` is the default)
    pub lang: &'a str,
    /// List of `.fbs` files to compile [required to be non-empty]
    pub inputs: &'a [&'a Path],
    /// Output path for the generated helpers (`-o PATH` parameter) [required]
    pub out_dir: &'a Path,
    /// Search for includes in the specified paths (`-I PATH` parameter)
    pub includes: &'a [&'a Path],
}

impl Default for Args<'_> {
    fn default() -> Self {
        Self {
            lang: "rust",
            out_dir: Path::new(""),
            includes: &[],
            inputs: &[],
        }
    }
}

/// Programmatic interface (API) for `flatc` command.
///
/// NOTE: You may only need a small helper function [`run`].
///
/// [`run`]: fn.run.html
pub struct Flatc {
    exec: PathBuf,
}

impl Flatc {
    /// New `flatc` command from `$PATH`
    pub fn from_env_path() -> Flatc {
        Flatc {
            exec: PathBuf::from("flatc"),
        }
    }

    /// New `flatc` command from specified path
    pub fn from_path(path: PathBuf) -> Flatc {
        Flatc {
            exec: path,
        }
    }

    /// Check `flatc` command found and valid
    pub fn check(&self) -> Result<()> {
        self.version().map(|_| ())
    }

    fn spawn(&self, cmd: &mut process::Command) -> io::Result<process::Child> {
        info!("spawning command {:?}", cmd);

        cmd.spawn()
            .map_err(|e| Error::new(e.kind(), format!("failed to spawn `{:?}`: {}", cmd, e)))
    }

    /// Obtain `flatc` version
    pub fn version(&self) -> Result<Version> {
        let child = self.spawn(
            process::Command::new(&self.exec)
                .stdin(process::Stdio::null())
                .stdout(process::Stdio::piped())
                .stderr(process::Stdio::piped())
                .args(&["--version"]),
        )?;

        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(err_other("flatc failed with error"));
        }
        let output = String::from_utf8(output.stdout).map_err(|e| err_other(e))?;
        let output = output
            .lines()
            .next()
            .ok_or_else(|| err_other("output is empty"))?;
        let prefix = "flatc version ";
        if !output.starts_with(prefix) {
            return Err(err_other("output does not start with prefix"));
        }
        let output = &output[prefix.len()..];
        let first_char = output
            .chars()
            .next()
            .ok_or_else(|| err_other("version is empty"))?;
        if !first_char.is_digit(10) {
            return Err(err_other("version does not start with digit"));
        }
        Ok(Version {
            version: output.to_owned(),
        })
    }

    /// Execute `flatc` command with given args, check it completed correctly.
    fn run_with_args(&self, args: Vec<OsString>) -> Result<()> {
        let mut cmd = process::Command::new(&self.exec);
        cmd.stdin(process::Stdio::null());
        cmd.args(args);

        let mut child = self.spawn(&mut cmd)?;

        if !child.wait()?.success() {
            return Err(err_other(format!(
                "flatc ({:?}) exited with non-zero exit code",
                cmd
            )));
        }

        Ok(())
    }

    /// Execute configured `flatc` with given args
    pub fn run(&self, args: Args) -> Result<()> {
        let mut cmd_args: Vec<OsString> = Vec::new();

        if args.out_dir.as_os_str().is_empty() {
            return Err(err_other("out_dir is empty"));
        }

        cmd_args.push({
            let mut arg = OsString::with_capacity(args.lang.len() + 3);
            arg.push("--");
            arg.push(args.lang);
            arg
        });

        if args.lang.is_empty() {
            return Err(err_other("lang is empty"));
        }

        cmd_args.push("-o".into());
        cmd_args.push(
            args.out_dir
                .to_str()
                .ok_or_else(|| {
                    Error::new(
                        io::ErrorKind::Other,
                        "only UTF-8 convertable paths are supported",
                    )
                })?
                .into(),
        );

        if args.inputs.is_empty() {
            return Err(err_other("input is empty"));
        }

        cmd_args.extend(args.inputs.iter().map(|input| input.into()));

        cmd_args.extend(args.includes.iter().map(|include| {
            let mut arg = OsString::with_capacity(include.as_os_str().len() + 3);
            arg.push("-I");
            arg.push(include.as_os_str());
            arg
        }));

        self.run_with_args(cmd_args)
    }
}

/// Execute `flatc` found in `$PATH` with given args
///
/// # Examples
///
/// Please, refer to [the root crate documentation](index.html#examples).
pub fn run(args: Args) -> Result<()> {
    let flatc = Flatc::from_env_path();

    // First check with have good `flatc`
    flatc.check()?;

    flatc.run(args)
}

/// FlatBuffers (flatc) version.
pub struct Version {
    version: String,
}

impl Version {
    pub fn version(&self) -> &str {
        &self.version
    }
}

#[cfg(test)]
mod test {
    use tempfile;

    use super::*;

    #[test]
    fn version() {
        Flatc::from_env_path().version().expect("version");
    }

    #[test]
    fn run_can_produce_output() -> io::Result<()> {
        let temp_dir = tempfile::Builder::new().prefix("flatc-rust").tempdir()?;
        let input_path = temp_dir.path().join("test.fbs");
        std::fs::write(&input_path, "table Test { text: string; } root_type Test;")
            .expect("test input fbs file could not be written");

        run(Args {
            lang: "rust",
            inputs: &[&input_path],
            out_dir: temp_dir.path(),
            ..Default::default()
        })
        .expect("run");

        let output_path = input_path.with_file_name("test_generated.rs");
        assert!(output_path.exists());
        assert_ne!(output_path.metadata().unwrap().len(), 0);

        Ok(())
    }
}
