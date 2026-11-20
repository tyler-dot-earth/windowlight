{
  description = "windowlight: sunlight through a window onto a wall, as a live Wayland wallpaper";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      # loaded at run time with dlopen
      runtimeLibs = pkgs: with pkgs; [ libglvnd wayland ];
    in {
      packages = forAll (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "windowlight";
          version = "0.1.0";
          src = ./native;
          cargoLock.lockFile = ./native/Cargo.lock;
          nativeBuildInputs = [ pkgs.makeWrapper ];
          postInstall = ''
            wrapProgram $out/bin/windowlight --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath (runtimeLibs pkgs)}
            for a in herdr system; do
              install -Dm755 adapters/$a $out/bin/windowlight-adapter-$a
              wrapProgram $out/bin/windowlight-adapter-$a --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.jq pkgs.gawk pkgs.coreutils ]}
            done
          '';
          meta.mainProgram = "windowlight";
        };
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.default ];
          packages = with pkgs; [ clippy rustfmt rust-analyzer jq nodejs glpaper chromium ];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (runtimeLibs pkgs);
        };
      });
    };
}
