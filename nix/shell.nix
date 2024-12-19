{pkgs}:
pkgs.mkShell {
  name = "rs-nc devShell";
  nativeBuildInputs = with pkgs; [
    # Compilers
    cargo
    rustc
    scdoc

    # build Deps
    pkg-config
    pango
    glib

    # Tools
    cargo-audit
    cargo-deny
    clippy
    rust-analyzer
    rustfmt
  ];
}