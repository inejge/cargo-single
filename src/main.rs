use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::{self, Command};

const USAGE: &str = r#"Usage:
    cargo-single <command> [<option> ...] {<source-file>|<source-dir>} [<arguments>]

<command> is one of: build, check, fmt, refresh, run
    "build", "check", "fmt" and "run" are regular Cargo subcommands.
    "refresh" will re-read the source file and update the dependencies in Cargo.toml.

<option> is one or more of:
    +<toolchain>                Name of a toolchain installed with Rustup.
    --release                   Build/check in release mode.
    --target <target>           Use the specified target for building.
    --no-quiet                  Don't pass --quiet to Cargo.

"fmt" will accept and forward all options to the real Cargo, even those which make
no sense for the subcommand."#;

fn fatal_exit(message: &str) -> ! {
    eprintln!("{}", message);
    process::exit(1);
}

#[derive(PartialEq, Eq, Hash)]
enum CargoOpts {
    Release,
    Target,
    Toolchain,
}

fn main() {
    let mut args = env::args();
    args.nth(1);
    let cmd = match args.next() {
        Some(cmd) => cmd,
        None => fatal_exit(USAGE),
    };
    let mut refresh_deps = false;
    match cmd.as_str() {
        "build" | "check" | "fmt" | "run" => (),
        "refresh" => refresh_deps = true,
        _ => fatal_exit(USAGE),
    }
    let mut cargo_args = vec![];
    let mut cargo_args_seen = HashSet::new();
    let mut rest = vec![];
    let mut is_quiet = true;
    let mut cargo_toolchain = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--no-quiet" => is_quiet = false,
            "--release" => {
                if cargo_args_seen.contains(&CargoOpts::Release) {
                    fatal_exit("cargo-single: --release already seen");
                }
                cargo_args_seen.insert(CargoOpts::Release);
                cargo_args.push(arg);
            }
            "--target" => {
                if cargo_args_seen.contains(&CargoOpts::Target) {
                    fatal_exit("cargo-single: --target already seen");
                }
                cargo_args_seen.insert(CargoOpts::Target);
                if let Some(target) = args.next() {
                    cargo_args.push(arg);
                    cargo_args.push(target);
                } else {
                    fatal_exit("cargo-single: --target needs an argument");
                }
            }
            toolchain if toolchain.starts_with("+") => {
                if cargo_args_seen.contains(&CargoOpts::Toolchain) {
                    fatal_exit("cargo-single: toolchain already set");
                }
                cargo_args_seen.insert(CargoOpts::Toolchain);
                cargo_toolchain = Some(arg);
            }
            _ => {
                rest.extend(args.collect::<Vec<_>>());
                rest.push(arg);
                break;
            }
        }
    }
    if rest.is_empty() {
        fatal_exit(USAGE);
    }
    let orig_src = rest.pop().expect("orig src");
    let mut src = PathBuf::from(&orig_src);
    let mut file_src = src.clone();
    match fs::metadata(&src) {
        Err(e) => {
            let mut passed = false;
            if src.extension().unwrap_or_default() != "rs" {
                file_src.set_extension("rs");
                if let Ok(md) = fs::metadata(&file_src) {
                    passed = md.is_file();
                }
            }
            if !passed {
                fatal_exit(&format!("cargo-single: fatal: {}: {}", orig_src, e));
            }
        }
        Ok(md) if md.is_dir() => {
            if !file_src.set_extension("rs") {
                fatal_exit(&format!(
                    "cargo-single: fatal: {}: cannot set extension",
                    orig_src
                ));
            }
            match fs::metadata(&file_src) {
                Err(e) => fatal_exit(&format!(
                    "cargo-single: fatal: {}: {}",
                    file_src.to_str().expect("source file"),
                    e
                )),
                Ok(md) if !md.is_file() => {
                    fatal_exit(&format!(
                        "cargo-single: fatal: {}: not a regular file",
                        file_src.to_str().expect("source file")
                    ));
                }
                _ => (),
            }
        }
        _ => (),
    }
    src.set_extension("");
    match fs::metadata(&src) {
        Ok(md) if !md.is_dir() => {
            fatal_exit(&format!(
                "cargo-single: fatal: {}: not a directory",
                src.to_str().expect("source dir")
            ));
        }
        Ok(_) => (),
        Err(_) => {
            let new_args = if is_quiet {
                &["new", "--quiet", "--bin"][..]
            } else {
                &["new", "--bin"][..]
            };
            match Command::new("cargo").args(new_args).arg(&src).status() {
                Err(e) => fatal_exit(&format!(
                    "cargo-single: error executing \"cargo new\": {}",
                    e
                )),
                Ok(status) if !status.success() => process::exit(1),
                _ => (),
            }
            let mut main_src = src.clone();
            main_src.push("src");
            main_src.push("main.rs");
            if let Err(e) = fs::remove_file(&main_src) {
                fatal_exit(&format!("cargo-single: error removing main.rs: {}", e));
            }
            if let Err(e) = fs::hard_link(&file_src, &main_src) {
                fatal_exit(&format!(
                    "cargo-single: error hardlinking to main.rs: {}",
                    e
                ));
            }
            refresh_deps = true;
        }
    }
    if refresh_deps {
        let mut cargo_path = src.clone();
        cargo_path.push("Cargo.toml");
        let mut cargo_tmp = src.clone();
        cargo_tmp.push(".Cargo.tmp");
        if let Err(e) = copy_deps(file_src, cargo_path, cargo_tmp) {
            fatal_exit(&format!(
                "cargo-single: error refreshing dependencies: {}",
                e
            ));
        }
    }
    match cmd.as_str() {
        "refresh" => return,
        "fmt" => cargo_args.clear(),
        _ => (),
    }
    if is_quiet {
        cargo_args.push("--quiet".to_owned());
    }
    cargo_args.push("--manifest-path".to_owned());
    src.push("Cargo.toml");
    cargo_args.push(src.to_str().expect("source dir").to_owned());
    let mut first_args = vec![];
    if let Some(toolchain) = cargo_toolchain.as_ref() {
        first_args.push(toolchain);
    }
    first_args.push(&cmd);
    match Command::new("cargo")
        .args(first_args)
        .args(&cargo_args)
        .arg("--")
        .args(&rest)
        .status()
    {
        Err(e) => fatal_exit(&format!(
            "cargo-single: error executing \"cargo {}\": {}",
            cmd, e
        )),
        Ok(status) if !status.success() => process::exit(status.code().unwrap_or(1)),
        _ => (),
    }
}

fn copy_deps(
    file_src: PathBuf,
    cargo_path: PathBuf,
    cargo_tmp: PathBuf,
) -> Result<(), Box<dyn Error>> {
    let src = File::open(&file_src)?;
    let src = BufReader::new(src);
    let cto = File::open(&cargo_path)?;
    let cto = BufReader::new(cto);
    let ctmp = File::create(&cargo_tmp)?;
    let mut ctmp = BufWriter::new(ctmp);
    let mut deps = String::new();
    let mut self_version = None;
    for src_line in src.lines() {
        let src_line = src_line?;
        if !src_line.starts_with("// ") {
            break;
        }
        if src_line.starts_with("// self = ") {
            self_version = Some(
                src_line
                    .splitn(2, "// self = ")
                    .nth(1)
                    .expect("version")
                    .to_owned(),
            );
            continue;
        }
        deps.push_str(src_line.splitn(2, "// ").nth(1).expect("rest of line"));
        deps.push('\n');
    }
    for cto_line in cto.lines() {
        let mut cto_line = cto_line?;
        if cto_line.starts_with("version = ") {
            if self_version.is_none() {
                continue;
            }
            cto_line = format!("version = {}", self_version.as_ref().unwrap());
        }
        dbg!(&cto_line);
        ctmp.write_all(cto_line.as_bytes())?;
        ctmp.write_all(b"\n")?;
        if cto_line == "[dependencies]" {
            ctmp.write_all(deps.as_bytes())?;
            break;
        }
    }
    ctmp.flush()?;
    drop(ctmp);
    fs::rename(&cargo_tmp, &cargo_path)?;
    Ok(())
}
