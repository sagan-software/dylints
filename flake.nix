{
  description = "Sagan's custom Rust lints for Dylint";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    dylint-src = {
      url = "github:trailofbits/dylint/v6.0.3";
      flake = false;
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      treefmt-nix,
      crane,
      rust-overlay,
      dylint-src,
      ...
    }:
    flake-utils.lib.eachSystem [ "aarch64-darwin" "aarch64-linux" "x86_64-linux" ] (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        inherit (pkgs) lib;
        inherit (flake-utils.lib) mkApp;
        version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
        rustupToolchain = "nightly-2026-07-15-${pkgs.stdenv.hostPlatform.rust.rustcTarget}";
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
          # The pinned compiler is supplied by crane. These libraries do not use
          # clippy_utils extra symbols; "nightly" avoids its network-only discovery.
          RUSTUP_TOOLCHAIN = "nightly-${pkgs.stdenv.hostPlatform.rust.rustcTarget}";
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
              for toolchain in ${rustupToolchain}; do
                driver_dir="$out/lib/dylint-drivers/$toolchain"
                mkdir -p "$driver_dir"
                cp target/release/sagan-dylint-driver "$driver_dir/dylint-driver"
              done
            '';
          }
        );

        source = lib.cleanSourceWith {
          src = ./.;
          filter =
            path: type:
            lib.cleanSourceFilter path type
            && !(
              type == "directory"
              && builtins.elem (baseNameOf path) [
                ".claude"
                ".direnv"
                ".rustup-dylint"
                "public"
                "result"
                "target"
              ]
            );
        };
        common = {
          pname = "sagan-dylint-workspace";
          inherit version;
          src = source;
          strictDeps = true;
          hardeningDisable = [ "fortify" ];
          nativeBuildInputs = [
            dylintTools
            pkgs.pkg-config
          ];
          buildInputs = [
            pkgs.openssl
            pkgs.zlib
          ];
          CARGO_PROFILE = "dev";
          CARGO_PROFILE_DEV_DEBUG = "0";
          CARGO_INCREMENTAL = "0";
          CARGO_BUILD_BUILD_DIR = "target";
          RUSTUP_TOOLCHAIN = rustupToolchain;
          DOCS_RS = "1";
        };
        cargoArtifacts = craneLib.buildDepsOnly (common // { doCheck = false; });
        libraryFilename = "libsagan_lints@${rustupToolchain}${pkgs.stdenv.hostPlatform.extensions.sharedLibrary}";
        lintLibraries = craneLib.buildPackage (
          common
          // {
            pname = "sagan-dylint-libraries";
            inherit cargoArtifacts;
            cargoExtraArgs = "-p sagan-lints";
            doCheck = false;
            installPhaseCommand = ''
              mkdir -p "$out/lib"
              install -m 0755 "$postBuildInstallFromCargoBuildLogOut/lib/libsagan_lints${pkgs.stdenv.hostPlatform.extensions.sharedLibrary}" "$out/lib/${libraryFilename}"
            '';
          }
        );
        catalog = craneLib.buildPackage (
          common
          // {
            pname = "sagan-lints-web";
            inherit cargoArtifacts;
            cargoExtraArgs = "-p sagan-lints-web";
            doCheck = false;
          }
        );
        setup = ''
          export RUSTUP_HOME="''${RUSTUP_HOME:-$PWD/.rustup-dylint}"
          export RUSTUP_TOOLCHAIN="${rustupToolchain}"
          export DYLINT_DRIVER_PATH="${dylintDriver}/lib/dylint-drivers"
          export CARGO_TARGET_${cargoTargetEnv}_LINKER=dylint-link
          # Dylint child builds select their own target directory for linker-named libraries.
          unset CARGO_BUILD_BUILD_DIR
          export OPENSSL_INCLUDE_DIR="${lib.getDev pkgs.openssl}/include"
          export OPENSSL_LIB_DIR="${lib.getLib pkgs.openssl}/lib"
          export PATH="${dylintTools}/bin:${pkgs.rustup}/bin:${rustToolchain}/bin:$PATH"
          mkdir -p "$RUSTUP_HOME"
          if [ ! -e "$RUSTUP_HOME/toolchains/${rustupToolchain}" ]; then
            mkdir -p "$RUSTUP_HOME/toolchains"
            ln -s ${rustToolchain} "$RUSTUP_HOME/toolchains/${rustupToolchain}"
          fi
        '';
        runtime = [
          dylintTools
          pkgs.git
          pkgs.gnugrep
          pkgs.pkg-config
          pkgs.rustup
          rustToolchain
          pkgs.mdbook
        ];
        command =
          name: text:
          pkgs.writeShellApplication {
            inherit name;
            runtimeInputs = runtime;
            text = setup + text;
          };
        task =
          name:
          command "sagan-${name}" ''
            exec cargo xtask ${name} "$@"
          '';
        selfLint = command "sagan-self-lint" ''
          export DYLINT_RUSTFLAGS="-D warnings"
          exec cargo dylint --no-metadata --no-build --lib-path ${lintLibraries}/lib/${libraryFilename} --workspace -- --lib --bins --tests "$@"
        '';
        checks = command "sagan-check" ''
          cargo fmt --all -- --check
          cargo clippy --workspace --lib --bins --tests -- -D warnings
          cargo test --workspace --lib --bins --tests
        '';
        docsSite =
          pkgs.runCommand "sagan-lints-site-${version}"
            {
              nativeBuildInputs = runtime ++ [ catalog ];
            }
            (
              setup
              + ''
                cargo dylint list --no-metadata --no-build --lib-path ${lintLibraries}/lib/${libraryFilename} > lints.txt
                sagan-lints-web --root ${source} --lint-list lints.txt --out-dir "$out"
                mdbook build ${source} --dest-dir "$out/book"
              ''
            );
        treefmt = treefmt-nix.lib.evalModule pkgs {
          projectRootFile = "flake.nix";
          programs = {
            rustfmt.enable = true;
            nixfmt.enable = true;
          };
          settings.global.excludes = [
            ".claude/**"
            ".direnv/**"
            ".git/**"
            ".rustup-dylint/**"
            "target/**"
            "public/**"
            "result/**"
          ];
        };
      in
      {
        packages = {
          default = lintLibraries;
          lint-libraries = lintLibraries;
          dylint-tools = dylintTools;
          docs-site = docsSite;
        };
        apps = {
          default = mkApp { drv = task "lint"; };
          self-lint = mkApp { drv = selfLint; };
          site = mkApp { drv = task "site"; };
          coverage = mkApp { drv = task "coverage"; };
          bench = mkApp { drv = task "bench"; };
          check = mkApp { drv = checks; };
          fmt = mkApp { drv = treefmt.config.build.wrapper; };
        };
        checks = {
          fmt = treefmt.config.build.check ./.;
          package = lintLibraries;
          docs-site = docsSite;
          clippy = craneLib.cargoClippy (
            common
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--workspace --lib --bins --tests -- -D warnings";
            }
          );
        };
        formatter = treefmt.config.build.wrapper;
        devShells.default = pkgs.mkShell {
          packages = runtime ++ [ treefmt.config.build.wrapper ];
          hardeningDisable = [ "fortify" ];
          shellHook = setup;
        };
      }
    );
}
