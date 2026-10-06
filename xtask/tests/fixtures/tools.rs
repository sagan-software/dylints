#![expect(
    unused_crate_dependencies,
    reason = "the fixture uses clap and shares xtask's other package dependencies"
)]
//! Child-process fixture for coverage orchestration; no shell scripts are required.
use clap::Parser;
use std::{env, ffi::OsString, fs, io::Write as _, path::PathBuf, process::ExitCode};

/// Select the invoked tool through its symlink name.
#[derive(Parser)]
#[command(multicall = true)]
struct Invocation {
    /// Tool selected by the caller.
    #[command(subcommand)]
    tool: Tool,
}

/// Executables emulated by this process fixture.
#[derive(clap::Subcommand)]
enum Tool {
    /// Cargo test, discovery and catalog operations.
    Cargo(Arguments),
    /// Compiler version and sysroot discovery.
    Rustc(Arguments),
    /// Coverage profile merging.
    #[command(name = "llvm-profdata")]
    Profdata(Arguments),
    /// Coverage data and HTML export.
    #[command(name = "llvm-cov")]
    Coverage(Arguments),
    /// Repository history and release references.
    Git(Arguments),
    /// GitHub release discovery and publication.
    Gh(Arguments),
    /// Repository book generation.
    Mdbook(Arguments),
}

/// Opaque tool arguments retained in their original order and encoding.
#[derive(clap::Args)]
#[command(disable_help_flag = true)]
struct Arguments {
    /// Arguments interpreted by the selected tool's fixture behavior.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 0..)]
    values: Vec<OsString>,
}

/// Recover the selected executable and its opaque arguments.
fn selected(tool: Tool) -> (&'static str, Arguments) {
    match tool {
        Tool::Cargo(arguments) => ("cargo", arguments),
        Tool::Rustc(arguments) => ("rustc", arguments),
        Tool::Profdata(arguments) => ("llvm-profdata", arguments),
        Tool::Coverage(arguments) => ("llvm-cov", arguments),
        Tool::Git(arguments) => ("git", arguments),
        Tool::Gh(arguments) => ("gh", arguments),
        Tool::Mdbook(arguments) => ("mdbook", arguments),
    }
}

/// Emulate Cargo and LLVM while leaving report interpretation to xtask.
fn main() -> ExitCode {
    let (tool, parsed) = selected(Invocation::parse().tool);
    let mut arguments = vec![OsString::from(tool)];
    arguments.extend(parsed.values);
    let root = PathBuf::from(env::var_os("FIXTURE_ROOT").unwrap());
    let mode = env::var("FIXTURE_MODE").unwrap_or_default();
    let mut calls = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("calls.txt"))
        .unwrap();
    writeln!(calls, "{tool} {arguments:?}").unwrap();
    match tool {
        "cargo" => {
            if arguments.get(1).is_some_and(|argument| argument != "test") {
                if mode == "command-fail" || mode == "wrapper-fail" {
                    return ExitCode::from(3);
                }
                if let Some(output) = arguments.windows(2).find(|pair| pair[0] == "--out-dir") {
                    let destination = PathBuf::from(&output[1]);
                    fs::create_dir_all(&destination).unwrap();
                    fs::write(destination.join("index.html"), "catalog fixture").unwrap();
                }
                return ExitCode::SUCCESS;
            }
            assert!(
                env::var("RUSTFLAGS")
                    .unwrap()
                    .contains("instrument-coverage")
            );
            assert_eq!(
                env::var_os("CARGO_TARGET_DIR"),
                env::var_os("CARGO_BUILD_BUILD_DIR")
            );
            let target = PathBuf::from(env::var_os("CARGO_TARGET_DIR").unwrap());
            fs::create_dir_all(target.join("debug/deps")).unwrap();
            if mode != "no-objects" {
                let _copied = fs::copy(
                    env::current_exe().unwrap(),
                    target.join("debug/deps/instrumented"),
                )
                .unwrap();
            }
            if mode == "extra-objects" || mode == "object-variants" {
                let _copied = fs::copy(
                    env::current_exe().unwrap(),
                    target.join("debug/deps/second"),
                )
                .unwrap();
            }
            fs::write(target.join("profiles/test.profraw"), "profile").unwrap();
            fs::write(root.join("test-arguments.txt"), format!("{arguments:?}")).unwrap();
            if mode == "tests-fail" {
                return ExitCode::from(7);
            }
        }
        "mdbook" => {
            if mode == "book-fail" {
                return ExitCode::from(3);
            }
            let output = arguments
                .windows(2)
                .find(|pair| pair[0] == "--dest-dir")
                .unwrap();
            let destination = PathBuf::from(&output[1]);
            fs::create_dir_all(&destination).unwrap();
            fs::write(destination.join("index.html"), "book fixture").unwrap();
        }
        "rustc" => {
            if arguments.iter().any(|argument| argument == "-vV") {
                if mode != "no-host" {
                    println!("host: fixture-host");
                }
            } else {
                let display = root.display();
                println!("{display}");
            }
        }
        "llvm-profdata" => {
            let output = arguments.windows(2).find(|pair| pair[0] == "-o").unwrap();
            fs::write(&output[1], "merged").unwrap();
        }
        "llvm-cov" => {
            eprintln!("fixture LLVM diagnostic");
            if mode == "llvm-fail" {
                return ExitCode::from(9);
            }
            if (mode == "text-fail" && arguments.get(1).is_some_and(|value| value == "report"))
                || (mode == "html-fail" && arguments.get(1).is_some_and(|value| value == "show"))
            {
                return ExitCode::from(9);
            }
            if arguments
                .iter()
                .any(|argument| argument == "--summary-only")
            {
                let source = fs::read_to_string(root.join("summary-input.json")).unwrap();
                print!("{source}");
            } else if arguments.iter().any(|argument| argument == "--format=lcov") {
                if mode == "canonical-write-fail" {
                    fs::create_dir_all(root.join("coverage/report/canonical_summary.json"))
                        .unwrap();
                }
                if mode == "object-variants" {
                    let second = arguments.iter().any(|argument| {
                        PathBuf::from(argument)
                            .file_name()
                            .is_some_and(|name| name == "second")
                    });
                    let path = root.join("src/shared.rs");
                    let path = path.display();
                    let first_hits = u8::from(!second);
                    let second_hits = u8::from(second);
                    println!("SF:{path}\nDA:10,{first_hits}\nDA:20,{second_hits}\nend_of_record");
                    return ExitCode::SUCCESS;
                }
                let source = fs::read_to_string(root.join("lcov-input.info")).unwrap();
                print!("{source}");
            } else if arguments.iter().any(|argument| argument == "show") {
                let output = arguments
                    .iter()
                    .find_map(|argument| argument.to_str().unwrap().strip_prefix("--output-dir="))
                    .unwrap();
                fs::write(
                    PathBuf::from(output).join("index.html"),
                    "<html><body>Coverage fixture</body></html>",
                )
                .unwrap();
            } else {
                println!("raw LLVM report");
            }
        }
        "git" => {
            if mode == "head-fail" && arguments.iter().any(|argument| argument == "HEAD") {
                return ExitCode::from(1);
            }
            if mode == "verify-spawn-fail" && arguments.iter().any(|argument| argument == "HEAD") {
                fs::remove_file(root.join("git")).unwrap();
            }
            if (mode == "tag-fail" && arguments.iter().any(|argument| argument == "tag"))
                || (mode == "tag-push-fail" && arguments.iter().any(|argument| argument == "push"))
                || (mode == "branch-read-fail"
                    && arguments.iter().any(|argument| argument == "ls-remote"))
                || (mode == "show-fail" && arguments.iter().any(|argument| argument == "show"))
            {
                return ExitCode::from(1);
            }
            if mode == "branch-race" {
                if arguments.iter().any(|argument| argument == "ls-remote") {
                    fs::write(root.join("branch-owner.txt"), "owner").unwrap();
                    return ExitCode::SUCCESS;
                }
                if arguments
                    .iter()
                    .any(|argument| argument.to_string_lossy().starts_with("HEAD:refs/heads/"))
                {
                    if arguments.iter().any(|argument| {
                        let argument = argument.to_string_lossy();
                        argument.starts_with("--force-with-lease=refs/heads/")
                            && argument.ends_with(':')
                    }) {
                        return ExitCode::from(1);
                    }
                    fs::write(root.join("branch-owner.txt"), "release").unwrap();
                }
            }
            if arguments.iter().any(|argument| argument == "rev-parse") {
                if arguments.iter().any(|argument| argument == "--verify") {
                    if mode == "existing-tag" || mode == "existing-release" {
                        println!("1111111111111111111111111111111111111111");
                    } else if mode == "conflicting-tag" {
                        println!("2222222222222222222222222222222222222222");
                    } else {
                        return ExitCode::from(1);
                    }
                } else {
                    println!("1111111111111111111111111111111111111111");
                }
            } else if arguments.iter().any(|argument| argument == "ls-remote")
                && mode == "existing-release"
            {
                println!("1111111111111111111111111111111111111111 refs/heads/release/0");
            } else if arguments.iter().any(|argument| argument == "show") {
                let version =
                    env::var("FIXTURE_BASE_VERSION").unwrap_or_else(|_| "0.1.0".to_owned());
                println!("[workspace.package]\nversion = {version:?}");
            }
        }
        "gh" => {
            if mode == "api-fail" {
                return ExitCode::from(4);
            }
            if mode == "existing-release" && arguments.iter().any(|argument| argument == "api") {
                let version = env::var("FIXTURE_VERSION").unwrap();
                println!("v{version}");
            }
        }
        _ => panic!("unknown fixture tool"),
    }
    ExitCode::SUCCESS
}
