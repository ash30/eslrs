let 
  pkgs = import <nixpkgs> { 
    overlays = [ 
      (import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"))
    ];
  };
  rustc = pkgs.rust-bin.stable.latest.default.override { extensions = ["rust-src"];};
  cargo = pkgs.rust-bin.stable.latest.default;
  rustPlatform = pkgs.makeRustPlatform { rustc = rustc; cargo = cargo;};
in

pkgs.mkShell {
  buildInputs = [
    cargo
    rustc
    pkgs.pkg-config
    pkgs.rust-bin.stable.latest.rust-analyzer # LSP Server
    pkgs.rust-bin.stable.latest.rustfmt       # Formatter
    pkgs.rust-bin.stable.latest.clippy        # Linter
  ];
  RUST_SRC_PATH = "${rustc}/lib/rustlib/src/rust/library/";
}


