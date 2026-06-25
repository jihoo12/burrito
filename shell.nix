{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    cargo
    rustc
    rustfmt
    clippy
    rust-analyzer

    wayland
    libxkbcommon
    vulkan-loader
  ];

  shellHook = ''
    export RUST_SRC_PATH=${pkgs.rustPlatform.rustLibSrc}
    
    export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${with pkgs; pkgs.lib.makeLibraryPath [
      wayland
      libxkbcommon
      vulkan-loader
    ]}"

    echo "🦀 Welcome to the Rust development environment! 🦀"
    rustc --version
  '';
}
