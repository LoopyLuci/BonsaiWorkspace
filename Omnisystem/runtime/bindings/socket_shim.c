// bindings/socket_shim.c — C wrapper for libc socket operations
// This minimal shim bridges Titan's socket_handler.ti FFI declarations
// to actual POSIX socket syscalls. Compile with: gcc -shared -fPIC socket_shim.c -o socket_shim.so

#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <unistd.h>
#include <string.h>
#include <errno.h>

// Forward declarations matching Titan FFI signatures
// Titan passes i64 (integers) for all parameters; we cast them as needed.

// Create a socket: socket(family, type, protocol)
long socket_impl(long family, long type, long protocol) {
    int fd = socket((int)family, (int)type, (int)protocol);
    return (long)fd;
}

// Bind socket to address: bind(fd, addr, addrlen)
// addr is a pointer to sockaddr (cast from i64)
long bind_impl(long fd, long addr_ptr, long addrlen) {
    struct sockaddr *addr = (struct sockaddr *)(uintptr_t)addr_ptr;
    int result = bind((int)fd, addr, (socklen_t)addrlen);
    return (long)result;
}

// Send packet: sendto(fd, buf, len, flags, dest_addr, addrlen)
long sendto_impl(long fd, long buf_ptr, long len, long flags, long dest_addr_ptr, long addrlen) {
    const void *buf = (const void *)(uintptr_t)buf_ptr;
    struct sockaddr *dest_addr = (struct sockaddr *)(uintptr_t)dest_addr_ptr;
    ssize_t bytes = sendto((int)fd, buf, (size_t)len, (int)flags, dest_addr, (socklen_t)addrlen);
    return (long)bytes;
}

// Receive packet: recvfrom(fd, buf, len, flags, src_addr, addrlen)
long recvfrom_impl(long fd, long buf_ptr, long len, long flags, long src_addr_ptr, long addrlen) {
    void *buf = (void *)(uintptr_t)buf_ptr;
    struct sockaddr *src_addr = (struct sockaddr *)(uintptr_t)src_addr_ptr;
    ssize_t bytes = recvfrom((int)fd, buf, (size_t)len, (int)flags, src_addr, (socklen_t *)&addrlen);
    return (long)bytes;
}

// Set socket option: setsockopt(fd, level, optname, optval, optlen)
long setsockopt_impl(long fd, long level, long optname, long optval, long optlen) {
    int value = (int)optval;
    int result = setsockopt((int)fd, (int)level, (int)optname, &value, (socklen_t)optlen);
    return (long)result;
}

// Close socket: close(fd)
long close_impl(long fd) {
    int result = close((int)fd);
    return (long)result;
}

// Helper: construct sockaddr_in from host and port
// Returns pointer to allocated sockaddr_in (cast to i64 for Titan)
long make_sockaddr_inet(long host_addr, long port) {
    struct sockaddr_in *addr = (struct sockaddr_in *)malloc(sizeof(struct sockaddr_in));
    if (!addr) return -1;

    memset(addr, 0, sizeof(struct sockaddr_in));
    addr->sin_family = AF_INET;
    addr->sin_port = htons((uint16_t)port);

    // Interpret host_addr as 4-byte IP (e.g., 192.168.1.1 = 0xc0a80101)
    addr->sin_addr.s_addr = htonl((uint32_t)host_addr);

    return (long)(uintptr_t)addr;
}

// Helper: free allocated sockaddr
void free_sockaddr(long addr_ptr) {
    free((void *)(uintptr_t)addr_ptr);
}
