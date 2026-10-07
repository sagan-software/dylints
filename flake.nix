{
  description = "Sagan's custom Rust lints for Dylint";

  # Request conservative limits where the Nix client controls build scheduling.
  # Multi-user daemons need the matching system-wide nix.conf settings.
  nixConfig = {
    max-jobs = 1;
    cores = 1;
  };

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
        toolchainChannel = (builtins.fromTOML (builtins.readFile ./rust-toolchain.toml)).toolchain.channel;
        rustupToolchain = "${toolchainChannel}-${pkgs.stdenv.hostPlatform.rust.rustcTarget}";
        cargoTargetEnv = lib.toUpper (
          lib.replaceStrings [ "-" ] [ "_" ] pkgs.stdenv.hostPlatform.rust.rustcTarget
        );
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = with pkgs; [
          openssl
          zlib
        ];
        dylintTools = craneLib.buildPackage {
          pname = "dylint-tools";
          version = "6.0.3";
          src = dylint-src;
          cargoExtraArgs = "-p cargo-dylint -p dylint-link";
          CARGO_BUILD_JOBS = "1";
          DOCS_RS = "1";
          inherit nativeBuildInputs buildInputs;
          doCheck = false;
        };

        dylintDriverSource = pkgs.runCommand "sagan-dylint-driver-source" { } ''
          cp -R ${dylint-src}/. "$out"
          chmod -R u+w "$out"
          cp -R ${./nix/dylint-driver} "$out/wrapper"
          chmod -R u+w "$out/wrapper"
          cp ${dylint-src}/driver/Cargo.lock "$out/wrapper/Cargo.lock"
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
          CARGO_BUILD_JOBS = "1";
          "CARGO_TARGET_${cargoTargetEnv}_LINKER" = "${pkgs.stdenv.cc}/bin/cc";
          RUSTFLAGS = "-C link-args=-Wl,-rpath,${rustToolchain}/lib";
          # Avoid clippy_utils symbol discovery, which otherwise fetches a driver.
          RUSTUP_TOOLCHAIN = "nightly-${pkgs.stdenv.hostPlatform.rust.rustcTarget}";
          inherit nativeBuildInputs buildInputs;
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

        common = {
          pname = "sagan-dylint-workspace";
          inherit version;
          # Flake inputs omit ignored local build outputs before Cargo sees the source.
          src = lib.cleanSource ./.;
          strictDeps = true;
          hardeningDisable = [ "fortify" ];
          nativeBuildInputs = [ dylintTools ] ++ nativeBuildInputs;
          inherit buildInputs;
          CARGO_PROFILE = "dev";
          CARGO_PROFILE_DEV_DEBUG = "0";
          CARGO_INCREMENTAL = "0";
          CARGO_BUILD_JOBS = "1";
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
        setup = ''
          export RUSTUP_HOME="''${RUSTUP_HOME:-$PWD/.rustup-dylint}"
          # The pinned compiler is read-only in /nix/store; Dylint asks rustup
          # to resolve rust-toolchain.toml while discovering workspace metadata.
          export RUSTUP_AUTO_INSTALL=0
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
          rustToolchain
          # dylint-link delegates the final compiler link to this `cc` wrapper.
          pkgs.stdenv.cc
          # The Rumdl lint's allocator dependency builds jemalloc with `make`.
          pkgs.gnumake
        ]
        ++ (with pkgs; [
          cargo-nextest
          git
          mdbook
          pkg-config
          rustup
        ]);
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
        treefmt = treefmt-nix.lib.evalModule pkgs {
          projectRootFile = "flake.nix";
          programs = {
            rustfmt.enable = true;
            nixfmt.enable = true;
            taplo.enable = true;
          };
          settings.global.excludes = [
            ".direnv/**"
            ".rustup-dylint/**"
            "target/**"
            "public/**"
            "result/**"
          ];
        };
        workspaceChecks = craneLib.mkCargoDerivation (
          common
          // {
            pname = "sagan-dylints-verification";
            inherit cargoArtifacts;
            buildPhaseCargoCommand = "true";
            # mkCargoDerivation leaves doCheck disabled unless requested.
            doCheck = true;
            checkPhaseCargoCommand = setup + ''
              cargo clippy --workspace --lib --bins --tests -- -D warnings -A unknown-lints
              cargo test --workspace --lib --bins --tests
              cargo test --workspace --doc
              export DYLINT_RUSTFLAGS="-D warnings"
              cargo dylint --no-metadata --no-build --lib-path ${lintLibraries}/lib/${libraryFilename} --workspace -- --lib --bins --tests
            '';
          }
        );
      in
      {
        packages = {
          default = lintLibraries;
          dylint-tools = dylintTools;
        };
        apps = {
          default = mkApp { drv = task "lint"; };
          site = mkApp { drv = task "site"; };
          coverage = mkApp { drv = task "coverage"; };
          bench = mkApp { drv = task "bench"; };
          fmt = mkApp { drv = treefmt.config.build.wrapper; };
        };
        checks = {
          fmt = treefmt.config.build.check ./.;
          verification = workspaceChecks;
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
