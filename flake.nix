{
  description = "Desktop GUI for editing the full surface of Hyprland configuration (Lua & conf)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (
        pkgs:
        let
          # Loaded at runtime by winit/wgpu (Wayland, xkb, Vulkan, GL).
          runtimeLibs = with pkgs; [
            wayland
            libxkbcommon
            vulkan-loader
            libGL
          ];
          hyprconf = pkgs.rustPlatform.buildRustPackage {
            pname = "hyprconf";
            version = "1.0.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = with pkgs; [
              pkg-config
              makeWrapper
            ];
            buildInputs = with pkgs; [
              libxkbcommon
              wayland
            ];

            postInstall = ''
              wrapProgram $out/bin/hyprconf \
                --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath runtimeLibs}
              install -Dm0644 packaging/hyprconf.desktop \
                $out/share/applications/hyprconf.desktop
            '';

            meta = with pkgs.lib; {
              description = "Desktop GUI for editing the full surface of Hyprland configuration (Lua & conf)";
              homepage = "https://github.com/hyprconf/hyprconf";
              license = with licenses; [
                mit
                asl20
              ];
              mainProgram = "hyprconf";
              platforms = platforms.linux;
            };
          };
        in
        {
          default = hyprconf;
          hyprconf = hyprconf;
        }
      );

      apps = forAllSystems (pkgs: {
        default = {
          type = "app";
          program = "${self.packages.${pkgs.system}.default}/bin/hyprconf";
        };
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            pkg-config
            just
          ];
          buildInputs = with pkgs; [
            libxkbcommon
            wayland
            vulkan-loader
            libGL
          ];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (
            with pkgs;
            [
              wayland
              libxkbcommon
              vulkan-loader
              libGL
            ]
          );
        };
      });
    };
}
