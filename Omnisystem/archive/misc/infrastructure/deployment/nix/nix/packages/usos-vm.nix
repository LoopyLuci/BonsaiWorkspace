# nix/packages/usos-vm.nix — Package USOS VM with QEMU launcher
{ stdenv, writeShellScriptBin, qemu, makeWrapper, usos-kernel, usos-initrd }:

stdenv.mkDerivation {
  pname = "usos-vm";
  version = "0.1.0";
  src = null;

  nativeBuildInputs = [ makeWrapper ];
  buildInputs = [ qemu ];

  installPhase = ''
    mkdir -p $out/bin $out/share/usos

    # Copy kernel and initrd
    cp ${usos-kernel}/share/usos/kernel.elf $out/share/usos/
    cp ${usos-initrd}/initrd.img $out/share/usos/

    # Create QEMU launcher script
    mkdir -p $out/bin
    cat > $out/bin/usos-vm-launcher.sh <<'LAUNCHER_SCRIPT'
#!/bin/bash
set -e

# Defaults
MEMORY="''${USOS_MEMORY:-2G}"
CPUS="''${USOS_CPUS:-2}"
KERNEL="''${USOS_KERNEL:-$1/share/usos/kernel.elf}"
INITRD="''${USOS_INITRD:-$1/share/usos/initrd.img}"
SHARE_DIR="''${USOS_SHARE_DIR:-/tmp/usos-shared}"
KVM_ENABLED=''${USOS_KVM:-true}

# Ensure shared directory exists
mkdir -p "$SHARE_DIR"

# Build QEMU command line
QEMU_ARGS=(
  # Machine type and CPU
  -machine q35,accel=$([ "$KVM_ENABLED" = true ] && echo kvm || echo tcg)
  -cpu host
  -m "$MEMORY"
  -smp "$CPUS"

  # Kernel and initrd
  -kernel "$KERNEL"
  -initrd "$INITRD"
  -append "console=ttyS0 root=/dev/vda1 rw"

  # Devices
  -nographic
  -serial mon:stdio
  -virtio-serial-pci

  # Network: user-mode NAT with SSH forwarding
  -netdev user,id=net0,hostfwd=tcp::2222-:22
  -device virtio-net-pci,netdev=net0

  # Shared folder via 9p (virtio)
  -fsdev local,security_model=passthrough,id=fsdev0,path="$SHARE_DIR"
  -device virtio-9p-pci,fsdev=fsdev0,mount_tag=hostshare

  # Additional optimizations
  -enable-kvm
  -no-reboot
  -snapshot
)

echo "USOS Co-OS Virtual Machine Launcher"
echo "===================================="
echo "Memory: $MEMORY"
echo "CPUs: $CPUS"
echo "Kernel: $KERNEL"
echo "Initrd: $INITRD"
echo "Shared folder: $SHARE_DIR"
echo "SSH forwarding: localhost:2222"
echo ""
echo "To connect: ssh -p 2222 root@localhost"
echo "To stop: Ctrl-C in the console"
echo "===================================="
echo ""

# Launch QEMU
exec ${qemu}/bin/qemu-system-x86_64 "''${QEMU_ARGS[@]}"
LAUNCHER_SCRIPT

    chmod +x $out/bin/usos-vm-launcher.sh

    # Create wrapper for easy invocation
    makeWrapper $out/bin/usos-vm-launcher.sh $out/bin/usos-vm \
      --add-flags "$out"
  '';

  meta = {
    description = "USOS Sentinel Core virtual machine with QEMU launcher";
    platforms = [ "x86_64-linux" "aarch64-linux" ];
  };
}
