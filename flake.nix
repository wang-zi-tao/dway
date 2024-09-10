{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    unstable.url = "github:nixos/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    # nixpkgs-qt5.url = "github:nixos/nixpkgs?rev=b3a285628a6928f62cdf4d09f4e656f7ecbbcafb";
  };

  # inputs.fenix.url = "github:nix-community/fenix";
  outputs = inputs@{ self, nixpkgs, unstable, fenix, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        fenix = inputs.fenix.packages.${system};
        rust-toolchain = fenix.complete;
        unstable = import inputs.unstable { inherit system pkgs; };
        nixpkgs-qt5 = import inputs.nixpkgs-qt5 { inherit system; };
        source-code = pkg: pkgs.stdenv.mkDerivation { src = pkg.src; };
        # qtbase = nixpkgs-qt5.libsForQt5.qt5.qtbase;
      in {
        devShell = pkgs.mkShell rec {
          nativeBuildInputs = with pkgs; [
            rust-toolchain.toolchain
            rust-toolchain.rust-analyzer

            mdbook
            mdbook-plantuml
            plantuml

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
            # unstable.wlroots_0_16
            wayland-protocols
            vulkan-tools
            hwdata
            glslang
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi

            glibc

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

            # libsForQt5.qt5.qtbase.dev
            # libsForQt5.qt5.qtdeclarative.dev
            # qt6.qtbase.dev
            # qt6.qtdeclarative.dev
            
          ];
          buildInputs = with pkgs; [
            tracy
            libinput
            seatd
            mesa
            udev
            alsaLib
            vulkan-loader
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
            (enableDebugging gtk4)
            gtk3.debug
            gnome2.pango
            gdk-pixbuf
            remarkable-toolchain
            xorg.libX11
            (enableDebugging cairo)
            (enableDebugging (gnome.gnome-calculator.override
              (attr: { gtk4 = enableDebugging gtk4; })))
            graphene
            xorg.libxcb
            # libsForQt5.qt5.qtbase
            # libsForQt5.qt5.qtdeclarative
            (buildEnv {
                name = "qt6";
                paths = with pkgs;[
                    (enableDebugging qt6.qtbase )
                    (enableDebugging qt6.qtdeclarative )
                ];
             })
            harfbuzz
            gvfs
            openssl
            pulseaudio

            # (gtk3)

            # lldb
            clang
          ];
          # LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs + "";
          AMD_VULKAN_ICD = "RADV";
          # AMD_VULKAN_ICD = "AMDVLK";
          # AMDVLK_ENABLE_DEVELOPING_EXT = "all";
          # VK_LOADER_DEBUG="all";
          # G_MESSAGES_DEBUG="all";
          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${
              pkgs.lib.makeLibraryPath buildInputs
            }"
          '';
        };
      });
}
