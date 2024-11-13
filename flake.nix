{
  description = "CLIPS.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    nil = {
      url = "github:oxalica/nil";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, nil, rust-overlay, ... }:
    let
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs {
        inherit overlays;
        system = "x86_64-linux";
      };

      clips-with-pc = pkgs.clips.overrideAttrs (final: previous: {
        installPhase = ''
          ${previous.installPhase}

          mkdir -p $out/lib/pkgconfig
          cat >> $out/lib/pkgconfig/clips.pc <<EOF
          Name: clips
          Description: CLIPS
          Version: 6.4.1
          Cflags: -I$out/include
          Libs: -L$out/lib -lclips
          EOF
        '';

        meta.pkgConfigModules = [ "clips" ];
      });
    in
    {
      devShells.x86_64-linux = {
        default = pkgs.mkShell {
          packages = with pkgs; [
            clips-with-pc

            (rust-bin.stable.latest.default.override
              {
                extensions = [ "rust-src" "rustfmt" "rust-analyzer" "clippy" ];
                targetExtensions = [ "rust-std" ];
                targets = [ "wasm32-unknown-unknown" ];
              })

            pkg-config
            clang

            # Both of these used with VSCode.
            nixpkgs-fmt
            nil.packages.${system}.default
          ];

          env = {
            RUST_BACKTRACE = "full";
            LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          };
        };
      };
    };
}
