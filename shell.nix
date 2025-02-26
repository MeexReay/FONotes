{ pkgs ? import <nixpkgs> {} }:
  pkgs.mkShell {
    nativeBuildInputs = with pkgs; [
      pkg-config
      xorg.libX11.dev
      xorg.libXft
      xorg.libXinerama
      xorg.libXi
      xorg.libXtst
      xorg.libX11 
      xorg.libXcursor 
      xorg.libXrandr
      libxkbcommon
      libxkbcommon.dev
      xorg.libxcb 
      alsa-lib
      libudev-zero
      openssl
    ];
    shellHook = ''
    	export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:${pkgs.libxkbcommon}/lib
    '';
}
