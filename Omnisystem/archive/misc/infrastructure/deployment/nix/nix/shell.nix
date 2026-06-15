# shell.nix — Development environment (legacy, use flake.nix instead)
{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    rustup
    cargo
    rustc
    rust-analyzer
    clang
    lld
    pkg-config
    openssl
    qemu
    libvirt
    nodejs_20
    pnpm
    sccache
  ];

  shellHook = ''
    export RUSTC_WRAPPER=sccache
    export RUSTFLAGS="-C target-cpu=native -C link-arg=-fuse-ld=lld"
    echo "🧬 Bonsai development environment loaded (shell.nix)"
  '';
}
