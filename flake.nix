{
  description = "Leet your code in command-line. Forked by yousiki.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    utils.url = "github:numtide/flake-utils";

    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      utils,
      naersk,
      rust-overlay,
      treefmt-nix,
      ...
    }:
    utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];

        pkgs = (import nixpkgs) {
          inherit system overlays;
        };

        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        naersk' = pkgs.callPackage naersk {
          cargo = toolchain;
          clippy = toolchain;
          rustc = toolchain;
        };

        nativeBuildInputs = with pkgs; [
          pkg-config
        ];

        darwinBuildInputs =
          with pkgs;
          lib.optionals stdenv.isDarwin [
            apple-sdk
          ];

        buildInputs =
          with pkgs;
          [
            dbus
            openssl
            sqlite
          ]
          ++ darwinBuildInputs;

        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);

        version = cargoToml.package.version;

        package = naersk'.buildPackage {
          inherit version buildInputs nativeBuildInputs;

          pname = "leetcode-cli";

          src = ./.;

          doCheck = true; # run `cargo test` on build

          cargoTestOptions =
            x:
            x
            ++ [
              "--all-features"
            ];

          nativeCheckInputs = with pkgs; [
            python3
          ];

          buildNoDefaultFeatures = true;

          buildFeatures = "git";

          meta = with pkgs.lib; {
            description = "Leet your code in command-line. Forked by yousiki.";
            homepage = "https://github.com/yousiki/leetcode-cli";
            licenses = licenses.mit;
            mainProgram = "leetcode";
          };

          # Env vars
          # a nightly compiler is required unless we use this cheat code.
          RUSTC_BOOTSTRAP = 0;

          # CFG_RELEASE = "${rustPlatform.rust.rustc.version}-stable";
          CFG_RELEASE_CHANNEL = "stable";
        };

        treefmtEval = treefmt-nix.lib.evalModule pkgs ./treefmt.nix;
      in
      {
        # Packages
        packages = {
          default = package;
          leetcode-cli = package;
        };

        # DevShells
        devShells.default =
          with pkgs;
          mkShell {
            name = "leetcode-cli-dev";
            inherit nativeBuildInputs;

            buildInputs = buildInputs ++ [
              toolchain
              cargo-about
              cargo-audit
              cargo-bloat
              cargo-edit
              cargo-outdated
            ];

            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
            RUST_BACKTRACE = "full";
            LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
          };

        # Formatters
        formatter = treefmtEval.config.build.wrapper;

        # Checks
        checks.formatting = treefmtEval.config.build.check self;
      }
    )
    // {
      # Overlays
      overlays.default = final: prev: {
        leetcode-cli = self.packages.${final.system}.leetcode-cli;
      };
    };
}
