{
  description = "Sui Tooling Version Manager";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
        };

        buildInputs =
          with pkgs;
          [
            openssl
            pkg-config
          ]
          ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
        ];

        # These libraries will be added to the RPATH of the patched binary
        runtimeLibs = with pkgs; [
          stdenv.cc.cc.lib # libstdc++.so.6, libgcc_s.so.1
          glibc # libc.so.6, libm.so.6, libpthread.so.0, libdl.so.2
          openssl # libssl.so, libcrypto.so (for reqwest with rustls-tls)
          zlib # libz.so.1 (for flate2)
        ];
      in
      let

        # Build the library path string
        patchData = (builtins.toJSON {
          lib_path = "${(pkgs.lib.makeLibraryPath runtimeLibs)}";
          interpreter = "${pkgs.glibc}/lib/ld-linux-x86-64.so.2";
        });

        # Import runtime dependencies configuration
        #runtimeDeps = import ./nix-runtime-deps.nix { inherit pkgs; };

        # Function to build suiup with optional patchelf
        mkSuiup =
          {
            enablePatchelf ? false,
          }:
          pkgs.rustPlatform.buildRustPackage {
            pname = "suiup";
            version = "0.0.4";

            inherit buildInputs patchData;

            src = ./.;
            # passAsFile = [ "patchData"];
            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            nativeBuildInputs =
              nativeBuildInputs
              ++ pkgs.lib.optionals enablePatchelf [
                pkgs.patchelf
              ];

            doCheck = false;

            passAsFile = [ "patchData" ];

            # Enable the nix-patchelf feature when building with patchelf
            buildFeatures = pkgs.lib.optionals enablePatchelf [ "nix-patchelf" ];

            postPatch = pkgs.lib.optionalString enablePatchelf ''
              substituteInPlace src/patchelf.rs \
                --replace-fail '"patchelf"' '"${pkgs.patchelf}/bin/patchelf"' \
                --replace-fail '/usr/share/suiup/nix-runtime-deps.json' "$out/share/suiup/nix-runtime-deps.json"
            '';

            # Install the runtime dependencies JSON file and patch suiup binary
            postInstall = pkgs.lib.optionalString enablePatchelf ''
              echo "Setting up Nix patchelf support..."

              # Create the data directory for runtime deps config
              mkdir -p $out/share/suiup
              cp $patchDataPath $out/share/suiup/nix-runtime-deps.json;
            '';

            meta = with pkgs.lib; {
              description = "Sui Tooling Version Manager";
              homepage = "https://github.com/Mystenlabs/suiup";
              license = licenses.asl20;
              maintainers = [ ];
              mainProgram = "suiup";
            };
          };

      in
      {
        packages = {
          # Default build without patchelf
          default = mkSuiup { enablePatchelf = true; };
        };

        devShells.default = pkgs.mkShell {
          inherit buildInputs;

          nativeBuildInputs =
            nativeBuildInputs
            ++ (with pkgs; [
              cargo-watch
              rust-analyzer
              patchelf
            ]);

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";

          # Set up XDG_DATA_HOME to point to a local directory for development
          shellHook = ''
            export XDG_DATA_HOME="''${XDG_DATA_HOME:-$HOME/.local/share}"
            echo "Nix development shell for suiup"
            echo "XDG_DATA_HOME: $XDG_DATA_HOME"
          '';
        };

        apps = {
          default = {
            type = "app";
            program = "${self.packages.${system}.default}/bin/suiup";
          };
        };
      }
    );
}
