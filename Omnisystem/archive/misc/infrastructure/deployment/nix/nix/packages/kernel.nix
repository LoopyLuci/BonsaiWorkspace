# nix/packages/usos-kernel.nix — Package the USOS microkernel
{ stdenv, sentinel-core }:

stdenv.mkDerivation {
  pname = "usos-kernel";
  version = "0.1.0";
  src = null;

  nativeBuildInputs = [ ];

  buildPhase = ''
    # The kernel is already built by the sentinel-core derivation
  '';

  installPhase = ''
    mkdir -p $out/bin $out/share/usos

    # Copy the bare-metal ELF kernel
    cp ${sentinel-core}/bin/sentinel-core $out/share/usos/kernel.elf

    # Verify it's a valid ELF for x86_64
    ${stdenv.cc.bintools.bintools}/bin/readelf -h $out/share/usos/kernel.elf > /dev/null

    # Create a convenience symlink
    ln -s $out/share/usos/kernel.elf $out/bin/usos-kernel

    # Metadata for QEMU/Multiboot2 compatibility
    mkdir -p $out/share/usos
    cat > $out/share/usos/info.txt <<'EOF'
    USOS Sentinel Core Kernel
    Architecture: x86_64
    Format: ELF (Multiboot2 compatible)
    Built from: ${sentinel-core}
    EOF
  '';

  meta = {
    description = "USOS Sentinel Core bare-metal microkernel";
    platforms = [ "x86_64-linux" "aarch64-linux" ];
  };
}
