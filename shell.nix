{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  packages = with pkgs; [
    rustc
    cargo
    pkg-config
    gtk4
    libadwaita
    glib
    gcc
    gnumake
  ];

  buildInputs = with pkgs; [
    gtk4
    libadwaita
    glib
  ];

  shellHook = ''
    export PKG_CONFIG_PATH="${pkgs.gtk4.dev}/lib/pkgconfig:${pkgs.libadwaita.dev}/lib/pkgconfig:${pkgs.glib.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
  '';
}
