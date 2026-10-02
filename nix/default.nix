{
  crane,
  dylint-src,
  lib,
  mkApp,
  pkgs,
  root,
}:
let
  rustupToolchain = "sagan-lints-dev-2026-07-15-${pkgs.stdenv.hostPlatform.rust.rustcTarget}";
  portableToolchain = "sagan-lints-2026-07-15-${pkgs.stdenv.hostPlatform.rust.rustcTarget}";
  cargoTargetEnv = lib.toUpper (
    lib.replaceStrings [ "-" ] [ "_" ] pkgs.stdenv.hostPlatform.rust.rustcTarget
  );
  rustToolchain = pkgs.rust-bin.nightly."2026-07-15".default.override {
    extensions = [
      "llvm-tools-preview"
      "rust-analyzer"
      "rustc-dev"
      "clippy"
      "rust-src"
    ];
  };
  craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

  dylintTools = craneLib.buildPackage {
    pname = "dylint-tools";
    version = "6.0.3";
    src = dylint-src;
    cargoExtraArgs = "-p cargo-dylint -p dylint-link";
    DOCS_RS = "1";
    nativeBuildInputs = [ pkgs.pkg-config ];
    buildInputs = [
      pkgs.openssl
      pkgs.zlib
    ];
    doCheck = false;
  };

  dylintDriverSource = pkgs.runCommand "sagan-dylint-driver-source" { } ''
    cp -R ${dylint-src}/. "$out"
    chmod -R u+w "$out"
    mkdir -p "$out/wrapper/src"
    cp ${dylint-src}/driver/Cargo.lock "$out/wrapper/Cargo.lock"
    cat >"$out/wrapper/Cargo.toml" <<'EOF'
    [package]
    name = "sagan-dylint-driver"
    version = "0.1.0"
    edition = "2024"

    [dependencies]
    dylint_driver = { path = "../driver" }

    [workspace]
    EOF
    cat >"$out/wrapper/src/main.rs" <<'EOF'
    #![feature(rustc_private)]

    fn main() {
        let args: Vec<_> = std::env::args_os().collect();
        if let Err(error) = dylint_driver::dylint_driver(&args) {
            eprintln!("{error:#}");
            std::process::exit(1);
        }
    }
    EOF
  '';
  dylintDriverCargoLock = dylintDriverSource + "/wrapper/Cargo.lock";
  dylintDriverCargoVendor = craneLib.vendorCargoDeps {
    src = dylintDriverSource;
    cargoLock = dylintDriverCargoLock;
  };
  dylintDriverCommonArgs = {
    pname = "sagan-dylint-driver";
    version = "0.1.0";
    src = dylintDriverSource;
    strictDeps = true;
    cargoLock = dylintDriverCargoLock;
    cargoVendorDir = dylintDriverCargoVendor;
    cargoExtraArgs = "--manifest-path wrapper/Cargo.toml";
    CARGO_TARGET_DIR = "target";
    "CARGO_TARGET_${cargoTargetEnv}_LINKER" = "${pkgs.stdenv.cc}/bin/cc";
    RUSTFLAGS = "-C link-args=-Wl,-rpath,${rustToolchain}/lib";
    RUSTUP_TOOLCHAIN = portableToolchain;
    nativeBuildInputs = [ pkgs.pkg-config ];
    buildInputs = [
      pkgs.openssl
      pkgs.zlib
    ];
    doCheck = false;
  };
  dylintDriverCargoArtifacts = craneLib.buildDepsOnly dylintDriverCommonArgs;
  dylintDriver = craneLib.buildPackage (
    dylintDriverCommonArgs
    // {
      cargoArtifacts = dylintDriverCargoArtifacts;
      doNotPostBuildInstallCargoBinaries = true;
      doNotRemoveReferencesToRustToolchain = true;
      installPhaseCommand = ''
        for toolchain in ${portableToolchain} ${rustupToolchain}; do
          driver_dir="$out/lib/dylint-drivers/$toolchain"
          mkdir -p "$driver_dir"
          cp target/release/sagan-dylint-driver "$driver_dir/dylint-driver"
        done
      '';
    }
  );

  workspaceRoot = toString (root.origSrc or root);
  # Lint UI fixtures, profiles, and catalog templates are not Cargo sources.
  sourceRoots = [
    "lints"
    "profiles"
    "scripts"
    "support"
    "tests"
    "web"
  ];
  isSourceRootPath =
    path: lib.any (name: lib.hasPrefix "${workspaceRoot}/${name}" (toString path)) sourceRoots;
  ignoredSourceRoots = [
    ".direnv"
    ".git"
    ".rustup-dylint"
    "public"
    "result"
    "target"
  ];
  isIgnoredSourcePath =
    path:
    let
      pathString = toString path;
    in
    lib.any (
      name:
      pathString == "${workspaceRoot}/${name}" || lib.hasPrefix "${workspaceRoot}/${name}/" pathString
    ) ignoredSourceRoots;

  lintSrc = lib.cleanSourceWith {
    src = root;
    filter =
      path: type:
      !isIgnoredSourcePath path
      && (
        (craneLib.filterCargoSources path type) || lib.hasSuffix "/README.md" path || isSourceRootPath path
      );
  };
  commonArgs = {
    pname = "rust-review-dylint-libraries";
    version = "0.1.0";
    src = lintSrc;
    strictDeps = true;
    # Keep Crane's reusable dependency artifacts in the same profile as the
    # packaged Dylint libraries. A profile mismatch rebuilds the full graph.
    CARGO_PROFILE = "dev";
    # Dylint loads these libraries dynamically and does not need DWARF. Full
    # dev-profile debug info makes the ARM build exceed the 20 GiB Linux
    # builder disk while linking the complete private lint suite.
    CARGO_PROFILE_DEV_DEBUG = "0";
    # jemalloc's configure probes use -O0 and promote glibc's fortify
    # warning to an error, so fortify cannot be enabled for this graph.
    hardeningDisable = [ "fortify" ];
    nativeBuildInputs = [
      dylintTools
      pkgs.pkg-config
    ];
    buildInputs = [
      pkgs.openssl
      pkgs.zlib
    ];
    RUSTUP_TOOLCHAIN = rustupToolchain;
  };

  categoryLibraries = {
    axum = {
      package = "axum";
      library = "axum";
    };
    bevy = {
      package = "bevy";
      library = "bevy";
    };
    cargo = {
      package = "cargo";
      library = "cargo";
    };
    clap = {
      package = "clap";
      library = "clap";
    };
    complexity = {
      package = "complexity";
      library = "complexity";
    };
    correctness = {
      package = "correctness";
      library = "correctness";
    };
    crates = {
      package = "crates";
      library = "crates";
    };
    insta = {
      package = "insta";
      library = "insta";
    };
    maintainability = {
      package = "maintainability";
      library = "maintainability";
    };
    perf = {
      package = "perf";
      library = "perf";
    };
    reqwest = {
      package = "reqwest-lints";
      library = "reqwest_lints";
    };
    restriction = {
      package = "restriction";
      library = "restriction";
    };
    schemars = {
      package = "schemars";
      library = "schemars";
    };
    serde = {
      package = "serde";
      library = "serde";
    };
    sqlx = {
      package = "sqlx";
      library = "sqlx";
    };
    test-case = {
      package = "test-case";
      library = "test_case";
    };
    style = {
      package = "style";
      library = "style";
    };
    suspicious = {
      package = "suspicious";
      library = "suspicious";
    };
    thiserror = {
      package = "thiserror";
      library = "thiserror";
    };
    tracing = {
      package = "tracing";
      library = "tracing";
    };
    tokio = {
      package = "tokio";
      library = "tokio";
    };
  };
  defaultLintLibraries = [
    "cargo"
    "complexity"
    "correctness"
    "crates"
    "maintainability"
    "perf"
    "restriction"
    "style"
    "suspicious"
  ];
  lintPackages = lib.unique (
    lib.mapAttrsToList (_: lintLibrary: lintLibrary.package) categoryLibraries
  );
  lintLibraryNames = lib.unique (
    lib.mapAttrsToList (_: lintLibrary: lintLibrary.library) categoryLibraries
  );
  lintCargoExtraArgs = lib.concatMapStringsSep " " (package: "-p ${package}@0.1.0") lintPackages;

  cargoArtifacts = craneLib.buildDepsOnly (
    commonArgs
    // {
      cargoExtraArgs = lintCargoExtraArgs;
      doCheck = false;
    }
  );

  lintLibraries = craneLib.buildPackage (
    commonArgs
    // {
      inherit cargoArtifacts;
      cargoExtraArgs = lintCargoExtraArgs;
      doCheck = false;
      installPhaseCommand = ''
        mkdir -p "$out/lib"
        for library in ${lib.concatStringsSep " " lintLibraryNames}; do
          library_path="$postBuildInstallFromCargoBuildLogOut/lib/lib$library${sharedLibrarySuffix}"
          if [ ! -f "$library_path" ]; then
            echo "missing aggregate lint library: $library_path" >&2
            exit 1
          fi
          install -m 0755 "$library_path" "$out/lib/"
        done
      '';
    }
  );

  sharedLibrarySuffix = pkgs.stdenv.hostPlatform.extensions.sharedLibrary;
  dylintLibraryFilename = library: "lib${library}@${rustupToolchain}${sharedLibrarySuffix}";
  dylintLibraryPath = pkgs.runCommand "rust-review-dylint-library-path-0.1.0" { } ''
    mkdir -p "$out/lib"
    ${lib.concatMapStringsSep "\n" (
      library:
      "cp ${lintLibraries}/lib/lib${library}${sharedLibrarySuffix} \"$out/lib/${dylintLibraryFilename library}\""
    ) lintLibraryNames}
  '';
  dylintLibraryArg = library: "--lib-path ${dylintLibraryPath}/lib/${dylintLibraryFilename library}";
  defaultDylintLibraryArgs = lib.concatMapStringsSep " " dylintLibraryArg defaultLintLibraries;
  saganLintsCommonArgs = {
    pname = "sagan-lints";
    version = "0.1.0";
    src = lintSrc;
    strictDeps = true;
    cargoExtraArgs = "-p sagan-lints";
    doNotRemoveReferencesToRustToolchain = true;
    "CARGO_TARGET_${cargoTargetEnv}_LINKER" = "${pkgs.stdenv.cc}/bin/cc";
    nativeBuildInputs = [ pkgs.pkg-config ];
    buildInputs = [
      pkgs.openssl
      pkgs.zlib
    ];
  };

  saganLintsCargoArtifacts = craneLib.buildDepsOnly (
    saganLintsCommonArgs
    // {
      doCheck = false;
    }
  );

  saganLints = craneLib.buildPackage (
    saganLintsCommonArgs
    // {
      cargoArtifacts = saganLintsCargoArtifacts;
      # Runner integration tests invoke Cargo against fixtures; CI runs them outside the sandbox.
      doCheck = false;
    }
  );

  siteCommonArgs = {
    pname = "sagan-lints-web";
    version = "0.1.0";
    src = lintSrc;
    strictDeps = true;
    cargoExtraArgs = "-p sagan-lints-web";
    "CARGO_TARGET_${cargoTargetEnv}_LINKER" = "${pkgs.stdenv.cc}/bin/cc";
  };

  siteCargoArtifacts = craneLib.buildDepsOnly (
    siteCommonArgs
    // {
      doCheck = false;
    }
  );

  siteXtask = craneLib.buildPackage (
    siteCommonArgs
    // {
      cargoArtifacts = siteCargoArtifacts;
      doCheck = false;
    }
  );

  # Lint levels come from the runner's registry; the README files supply the documentation.
  listRegisteredLints = pkgs.writeShellApplication {
    name = "list-registered-lints";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.git
      saganLints
    ];
    text = ''
      scratch="$(mktemp -d)"
      trap 'rm -rf -- "$scratch"' EXIT
      mkdir -p "$scratch/repo"
      printf '[workspace]\nresolver = "2"\nmembers = []\n' >"$scratch/repo/Cargo.toml"
      git -C "$scratch/repo" init -q
      HOME="$scratch/home" SAGAN_LINTS_CACHE_DIR="$scratch/cache" \
        sagan-lints --repo "$scratch/repo" --list-private-lints
    '';
  };

  docsSite =
    pkgs.runCommand "sagan-lints-site"
      {
        nativeBuildInputs = [
          listRegisteredLints
          siteXtask
        ];
      }
      ''
        list-registered-lints >lints.txt
        sagan-lints-web --root ${root} --lint-list lints.txt --out-dir "$out"
      '';

  generateSite = pkgs.writeShellApplication {
    name = "generate-lint-site";
    runtimeInputs = [
      listRegisteredLints
      siteXtask
    ];
    text = ''
      out_dir="''${1:-public}"
      if [ "$#" -gt 1 ]; then
        echo "usage: nix run .#site -- [OUT_DIR]" >&2
        exit 2
      fi
      lint_list="$(mktemp)"
      trap 'rm -f -- "$lint_list"' EXIT
      list-registered-lints >"$lint_list"
      sagan-lints-web --root "$PWD" --lint-list "$lint_list" --out-dir "$out_dir"
    '';
  };

  setupRustupToolchain = ''
    is_private_lints_root() {
      [ -f "$1/Cargo.toml" ] \
        && grep -q '^\[workspace.metadata.dylint\]' "$1/Cargo.toml"
    }

    original_private_lints_root="''${PRIVATE_LINTS_ROOT:-}"
    if [ -z "$original_private_lints_root" ] \
      || ! is_private_lints_root "$original_private_lints_root"; then
      if is_private_lints_root "$PWD"; then
        export PRIVATE_LINTS_ROOT="$PWD"
        if [ -n "$original_private_lints_root" ]; then
          case "''${RUSTUP_HOME:-}" in
            "$original_private_lints_root"/*) unset RUSTUP_HOME ;;
          esac
          case "''${DYLINT_DRIVER_PATH:-}" in
            "$original_private_lints_root"/*) unset DYLINT_DRIVER_PATH ;;
          esac
        fi
      else
        export PRIVATE_LINTS_ROOT="$original_private_lints_root"
      fi
    else
      export PRIVATE_LINTS_ROOT="$original_private_lints_root"
    fi

    if ! is_private_lints_root "$PRIVATE_LINTS_ROOT"; then
      echo "PRIVATE_LINTS_ROOT must point to this private lint suite checkout" >&2
      echo "current value: $PRIVATE_LINTS_ROOT" >&2
      exit 1
    fi
    export RUSTUP_HOME="''${RUSTUP_HOME:-$PRIVATE_LINTS_ROOT/.rustup-dylint}"
    export RUSTUP_TOOLCHAIN="${rustupToolchain}"
    export DYLINT_DRIVER_PATH="''${DYLINT_DRIVER_PATH:-${dylintDriver}/lib/dylint-drivers}"
    export CARGO_TARGET_${cargoTargetEnv}_LINKER=dylint-link
    export OPENSSL_INCLUDE_DIR="${lib.getDev pkgs.openssl}/include"
    export OPENSSL_LIB_DIR="${lib.getLib pkgs.openssl}/lib"
    export PKG_CONFIG_PATH="${lib.getDev pkgs.openssl}/lib/pkgconfig:''${PKG_CONFIG_PATH:-}"
    export PATH="${dylintTools}/bin:${pkgs.rustup}/bin:${rustToolchain}/bin:$PATH"

    mkdir -p "$RUSTUP_HOME" "$DYLINT_DRIVER_PATH"
    if ! rustup toolchain list | grep -Eq '^${rustupToolchain}($| )'; then
      rustup toolchain link "${rustupToolchain}" ${rustToolchain}
    fi
  '';

  dylintRuntimeInputs = [
    dylintTools
    pkgs.gnugrep
    pkgs.pkg-config
    pkgs.rustup
    rustToolchain
  ];

  withRepoSetup =
    text:
    ''
      set -euo pipefail
      ${setupRustupToolchain}
    ''
    + text;

  clippyCheck = craneLib.cargoClippy (
    commonArgs
    // {
      inherit cargoArtifacts;
      cargoClippyExtraArgs = "--workspace --lib --bins --tests -- -D warnings";
    }
  );

  runClippy = pkgs.writeShellApplication {
    name = "run-clippy";
    runtimeInputs = dylintRuntimeInputs;
    text = withRepoSetup ''
      cd "$PRIVATE_LINTS_ROOT"
      cargo clippy --workspace --lib --bins --tests -- -D warnings
    '';
  };

  runTests = pkgs.writeShellApplication {
    name = "run-rust-tests";
    runtimeInputs = dylintRuntimeInputs;
    text = withRepoSetup ''
      cd "$PRIVATE_LINTS_ROOT"
      exec cargo test "$@"
    '';
  };

  runChecks = pkgs.writeShellApplication {
    name = "run-rust-checks";
    runtimeInputs = dylintRuntimeInputs;
    text = withRepoSetup ''
      cd "$PRIVATE_LINTS_ROOT"
      cargo fmt --all -- --check
      cargo clippy --workspace --lib --bins --tests -- -D warnings
      cargo test --workspace --lib --bins --tests
    '';
  };

  runCoverage = pkgs.writeShellApplication {
    name = "run-coverage";
    runtimeInputs = dylintRuntimeInputs ++ [
      pkgs.coreutils
      pkgs.findutils
      pkgs.gawk
      pkgs.git
      pkgs.gnused
      pkgs.jq
      pkgs.stdenv.cc
    ];
    text = withRepoSetup ''
      cd "$PRIVATE_LINTS_ROOT"
      exec bash ${root}/scripts/coverage.sh "$@"
    '';
  };

  listLints = pkgs.writeShellApplication {
    name = "list-rust-lints";
    runtimeInputs = dylintRuntimeInputs;
    text = withRepoSetup ''
      exec cargo dylint list \
        --no-metadata \
        --no-build \
        ${defaultDylintLibraryArgs}
    '';
  };

  runSelfLints = pkgs.writeShellApplication {
    name = "run-self-lints";
    runtimeInputs = dylintRuntimeInputs;
    text = withRepoSetup ''
      cd "$PRIVATE_LINTS_ROOT"
      # Lint library, binary, and test targets; UI example targets are lint fixtures.
      exec cargo run --quiet --bin sagan-lints -- \
        --repo . \
        --heartbeat-seconds 0 \
        --use-repo-clippy-config \
        --no-all-targets \
        --extra-cargo-arg=--lib \
        --extra-cargo-arg=--bins \
        --extra-cargo-arg=--tests \
        "$@"
    '';
  };

  runPrivateLints = pkgs.writeShellApplication {
    name = "run-private-lints";
    runtimeInputs = dylintRuntimeInputs;
    text = withRepoSetup ''
      export CARGO_INCREMENTAL=0
      exec cargo dylint \
        --no-metadata \
        --no-build \
        ${defaultDylintLibraryArgs} \
        "$@"
    '';
  };

  runCategoryLints =
    category: lintLibrary:
    pkgs.writeShellApplication {
      name = "run-${category}-lints";
      runtimeInputs = dylintRuntimeInputs;
      text = withRepoSetup ''
        export CARGO_INCREMENTAL=0
        exec cargo dylint \
          --no-metadata \
          --no-build \
          ${dylintLibraryArg lintLibrary.library} \
          "$@"
      '';
    };

  categoryApps = lib.mapAttrs (
    category: lintLibrary: mkApp { drv = runCategoryLints category lintLibrary; }
  ) categoryLibraries;
in
{
  inherit
    saganLints
    listLints
    runChecks
    runTests
    setupRustupToolchain
    ;

  devShellPackages = dylintRuntimeInputs;
  treefmt = {
    programs.rustfmt.enable = true;
    excludes = [ ".rustup-dylint/**" ];
  };

  flake = {
    packages = {
      docs-site = docsSite;
      dylint-library-path = dylintLibraryPath;
      dylint-tools = dylintTools;
      sagan-lints = saganLints;
      lint-libraries = lintLibraries;
      runner = runPrivateLints;
    };

    apps = categoryApps // {
      clippy = mkApp { drv = runClippy; };
      self-lint = mkApp { drv = runSelfLints; };
      site = mkApp { drv = generateSite; };
      test = mkApp { drv = runTests; };
      check = mkApp { drv = runChecks; };
      coverage = mkApp { drv = runCoverage; };
    };

    checks = {
      clippy = clippyCheck;
      docs-site = docsSite;
      package = lintLibraries;
    };
  };
}
