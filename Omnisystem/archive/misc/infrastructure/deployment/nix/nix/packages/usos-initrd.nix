# nix/packages/usos-initrd.nix — Build USOS initrd with Weave and boot services
{ stdenv, bonsai-weave, bonsai-workspace, busybox, cpio, gzip }:

stdenv.mkDerivation {
  pname = "usos-initrd";
  version = "0.1.0";
  src = null;

  nativeBuildInputs = [ cpio gzip ];

  buildPhase = ''
    mkdir -p rootfs/{bin,dev,proc,sys,etc,lib,usr/share,mnt/host,var/lib/bonsai}

    # Busybox for minimal utilities
    cp ${busybox}/bin/busybox rootfs/bin/
    for cmd in sh ls cat mount umount mkdir ln rm cp; do
      ln -s busybox rootfs/bin/$cmd || true
    done

    # Weave component manager
    cp ${bonsai-weave}/bin/bonsai-weave rootfs/bin/weave

    # Bonsai Workspace (optional GUI, requires Wayland support)
    cp ${bonsai-workspace}/bin/bonsai-workspace rootfs/bin/ || true

    # Essential directories
    ln -s bin rootfs/sbin || true
    ln -s lib rootfs/lib64 || true

    # Minimal libc library
    mkdir -p rootfs/lib
    cp ${busybox}/lib/ld*.so* rootfs/lib/ || true

    # Init script - starts Weave and keeps VM alive
    cat > rootfs/init <<'INIT_SCRIPT'
#!/bin/sh
set -e

# Mount kernel filesystems
mount -t proc none /proc
mount -t sysfs none /sys
mount -t devtmpfs none /dev

# Create essential device nodes
mknod /dev/null c 1 3 2>/dev/null || true
mknod /dev/zero c 1 5 2>/dev/null || true
mknod /dev/tty c 5 0 2>/dev/null || true

# Mount host shared folder (virtio-9p)
if [ -d /sys/module/9pnet_virtio ]; then
  mount -t 9p -o trans=virtio hostshare /mnt/host 2>/dev/null || true
fi

echo "USOS Sentinel Core — booting..."
echo "========================================"

# Start Weave component system
echo "Starting Weave..."
/bin/weave &
WEAVE_PID=$!

# Optionally start Bonsai Workspace (requires Wayland, usually not available in VM)
# /bin/bonsai-workspace &

# Wait for critical services
sleep 1

# Interactive shell on failure, keeps VM alive
if [ ! -d /proc/self ]; then
  echo "FATAL: Weave failed to start"
  exec /bin/sh
else
  echo "USOS running. PID $WEAVE_PID"
  # Wait for Weave indefinitely
  wait $WEAVE_PID || exec /bin/sh
fi
INIT_SCRIPT

    chmod +x rootfs/init

    # Create device nodes
    mkdir -p rootfs/dev
    mknod rootfs/dev/null c 1 3 2>/dev/null || true
    mknod rootfs/dev/zero c 1 5 2>/dev/null || true

    # Package the rootfs as cpio.gz (standard initrd format)
    (cd rootfs && find . -print0 | cpio -0 -H newc -o | gzip -9) > initrd.cpio.gz
  '';

  installPhase = ''
    mkdir -p $out
    cp initrd.cpio.gz $out/initrd.img
  '';

  meta = {
    description = "USOS minimal initrd with Weave and essential services";
  };
}
