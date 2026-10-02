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
    flake-utils.lib.eachSystem
      [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ]
      (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };

          inherit (pkgs) lib;
          inherit (flake-utils.lib) mkApp;
          rust = pkgs.callPackage ./nix {
            inherit
              crane
              dylint-src
              mkApp
              pkgs
              ;
            root = lib.cleanSourceWith {
              src = ./.;
              filter =
                path: type:
                lib.cleanSourceFilter path type
                && !(
                  type == "directory"
                  && builtins.elem (baseNameOf path) [
                    ".direnv"
                    "public"
                    "result"
                    "target"
                  ]
                );
            };
          };

          packagedRunnerOfflineCheck =
            pkgs.runCommand "sagan-lints-packaged-runner-offline-test"
              {
                nativeBuildInputs = [ pkgs.git ];
              }
              ''
                mkdir -p "$TMPDIR/repo"
                cat >"$TMPDIR/repo/Cargo.toml" <<'EOF'
                [workspace]
                resolver = "2"
                members = []
                EOF
                git -C "$TMPDIR/repo" init -q

                export CARGO_HOME="$TMPDIR/cargo"
                export CARGO_NET_OFFLINE=true
                export HOME="$TMPDIR/home"
                export SAGAN_LINTS_CACHE_DIR="$TMPDIR/cache"
                ${rust.saganLints}/bin/sagan-lints \
                  --repo "$TMPDIR/repo" \
                  --list-private-lints >"$TMPDIR/list.txt" 2>&1

                if grep -Fq 'Building driver' "$TMPDIR/list.txt"; then
                  cat "$TMPDIR/list.txt" >&2
                  echo "packaged runner tried to rebuild the Dylint driver" >&2
                  exit 1
                fi
                grep -Fq 'string_error_result' "$TMPDIR/list.txt"
                touch "$out"
              '';

          treefmtModule = {
            projectRootFile = "flake.nix";
            programs = rust.treefmt.programs // {
              nixfmt.enable = true;
            };
            settings.global.excludes = [
              ".direnv/**"
              ".git/**"
              "target/**"
              "result/**"
            ]
            ++ rust.treefmt.excludes;
          };
          treefmtEval = treefmt-nix.lib.evalModule pkgs treefmtModule;
        in
        {
          packages = rust.flake.packages // {
            default = rust.saganLints;
          };

          apps = rust.flake.apps // {
            default = mkApp { drv = rust.saganLints; };
            fmt = mkApp { drv = treefmtEval.config.build.wrapper; };
          };

          checks = rust.flake.checks // {
            fmt = treefmtEval.config.build.check ./.;
            packaged-runner-offline = packagedRunnerOfflineCheck;
          };

          formatter = treefmtEval.config.build.wrapper;

          devShells.default = pkgs.mkShell {
            packages = rust.devShellPackages ++ [ treefmtEval.config.build.wrapper ];
            # jemalloc's configure probes use -O0 and promote glibc's fortify warning to an error.
            hardeningDisable = [ "fortify" ];
            shellHook = rust.setupRustupToolchain;
          };
        }
      );
}
