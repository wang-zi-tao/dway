{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    unstable.url = "github:nixos/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  # inputs.fenix.url = "github:nix-community/fenix";
  outputs = inputs@{ self, nixpkgs, unstable, fenix, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        fenix = inputs.fenix.packages.${system};
        unstable = import inputs.unstable {
          inherit system pkgs;
        };
        source-code = pkg: pkgs.stdenv.mkDerivation {
          src = pkg.src;
        };
      in
      {
        devShell = pkgs.mkShell rec{
          nativeBuildInputs = with pkgs; [
            (fenix.fromToolchainFile {
              file = ./rust-toolchain.toml;
              sha256 = "sha256-lI+VFFbRie8hMZHqzFXq69ebobNZ8Q/53czHXzzZINk=";
            })
            tracy
            cargo-flamegraph
            pkg-config
            libcxx
            gcc-unwrapped
            nx-libs
            libinput
            libudev-zero
            meson
            pixman
            (enableDebugging xwayland)
            xorg.libX11
            xorg.xcbutilimage
            xorg.xcbutilwm
            mesa
            unstable.wlroots_0_16
            wayland-protocols
            vulkan-tools
            hwdata
            glslang
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi

            glibc

            llvmPackages.bintools # To use lld linker
            egl-wayland.dev
            libglvnd.dev
            glew-egl.dev
            libinput.dev
            libxkbcommon.dev
            seatd.dev
            wayland.dev
            xorg.libxcb.dev
            dbus.dev
            libdrm.dev
            xorg.xcbutilrenderutil.dev
            xorg.xcbutilerrors.dev
            libpng.dev
            ffmpeg.dev
            alsa-lib.dev
            fontconfig.dev
            udev
            glibc.dev
          ];
          buildInputs = with pkgs;[
            tracy
            libinput
            seatd
            mesa
            udev
            alsaLib
            (enableDebugging vulkan-loader )
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi # To use x11 feature
            libxkbcommon
            wayland # To use wayland feature
            libglvnd
            dbus.lib
            fontconfig.lib
            freetype
            libglvnd
            (glib)
            (gtk4)
            gtk3.debug
            gnome2.pango
            gdk-pixbuf
            remarkable-toolchain
            xorg.libX11
            (cairo)
            (gnome.gedit)
            graphene
            xorg.libxcb
            libsForQt5.qt5.qtbase
            harfbuzz
            gvfs
            openssl
            pulseaudio

            # (gtk3)

            # lldb
          ];
          # LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs + "";
          AMD_VULKAN_ICD = "RADV";
          # AMD_VULKAN_ICD = "AMDVLK";
          AMDVLK_ENABLE_DEVELOPING_EXT="all";
          # VK_LOADER_DEBUG="all";
          # G_MESSAGES_DEBUG="all";
          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath buildInputs}"
          '';
        };
      });
}
