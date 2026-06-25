/**
 * Omnisystem Java/JNI Bindings
 *
 * Provides Java access to Omnisystem kernel via Java Native Interface (JNI).
 * Enables enterprise integration with JVM-based services (Spring Boot, Kafka, etc).
 *
 * Usage:
 *   OmniKernel kernel = new OmniKernel();
 *   kernel.initialize();
 *   SystemStats stats = kernel.getStats();
 *   long pid = kernel.createProcess();
 *
 * Compilation (requires omnisystem-go-bindings built):
 *   javac Omnisystem.java
 *
 * Execution:
 *   java -Djava.library.path=/path/to/target/release Omnisystem
 */

public class Omnisystem {
    // Native library loading
    static {
        try {
            System.loadLibrary("omnisystem_go");
        } catch (UnsatisfiedLinkError e) {
            System.err.println("Failed to load omnisystem_go native library");
            System.err.println("Make sure omnisystem-go-bindings is built with: cargo build --release");
            System.err.println("And set: java -Djava.library.path=/path/to/target/release");
            throw new ExceptionInInitializerError(e);
        }
    }

    // Native method declarations (JNI calls)
    private native int nativeInit();
    private native long nativeGetTotalMemory();
    private native long nativeGetAllocatedMemory();
    private native long nativeGetFreeMemory();
    private native int nativeGetProcessCount();
    private native long nativeCreateProcess();
    private native int nativeRegisterModule(String name, int major, int minor, int patch);
    private native int nativeGetHealth();
    private native int nativeEchoInt(int value);
    private native long nativeEchoU64(long value);
    private native int nativeShutdown();

    // Instance state
    private boolean initialized = false;

    /**
     * Initialize Omnisystem kernel
     * @throws OmnisystemException if initialization fails
     */
    public void initialize() throws OmnisystemException {
        System.out.println("📦 Initializing Omnisystem kernel from Java...");

        int result = nativeInit();
        if (result == 0) {
            initialized = true;
            System.out.println("✓ Omnisystem kernel initialized successfully\n");
        } else {
            throw new OmnisystemException(String.format("Failed to initialize kernel (error code: %d)", result));
        }
    }

    /**
     * Get total system memory in bytes
     * @return total memory bytes
     * @throws OmnisystemException if kernel not initialized
     */
    public long getTotalMemory() throws OmnisystemException {
        checkInitialized();
        return nativeGetTotalMemory();
    }

    /**
     * Get allocated memory in bytes
     * @return allocated memory bytes
     * @throws OmnisystemException if kernel not initialized
     */
    public long getAllocatedMemory() throws OmnisystemException {
        checkInitialized();
        return nativeGetAllocatedMemory();
    }

    /**
     * Get free memory in bytes
     * @return free memory bytes
     * @throws OmnisystemException if kernel not initialized
     */
    public long getFreeMemory() throws OmnisystemException {
        checkInitialized();
        return nativeGetFreeMemory();
    }

    /**
     * Get number of processes
     * @return process count
     * @throws OmnisystemException if kernel not initialized
     */
    public int getProcessCount() throws OmnisystemException {
        checkInitialized();
        return nativeGetProcessCount();
    }

    /**
     * Create a new process
     * @return process ID, throws exception on failure
     * @throws OmnisystemException if creation fails or kernel not initialized
     */
    public long createProcess() throws OmnisystemException {
        checkInitialized();
        long pid = nativeCreateProcess();
        if (pid == 0) {
            throw new OmnisystemException("Failed to create process");
        }
        return pid;
    }

    /**
     * Register an FFI module
     * @param name module name
     * @param major major version
     * @param minor minor version
     * @param patch patch version
     * @throws OmnisystemException if registration fails
     */
    public void registerModule(String name, int major, int minor, int patch) throws OmnisystemException {
        checkInitialized();
        int result = nativeRegisterModule(name, major, minor, patch);
        if (result != 0) {
            throw new OmnisystemException(String.format("Failed to register module %s", name));
        }
    }

    /**
     * Get system health status
     * @return "healthy", "degraded", or "critical"
     * @throws OmnisystemException if kernel not initialized
     */
    public String getHealth() throws OmnisystemException {
        checkInitialized();
        int status = nativeGetHealth();
        switch (status) {
            case 0:
                return "healthy";
            case 1:
                return "degraded";
            case 2:
                return "critical";
            default:
                return "unknown";
        }
    }

    /**
     * Echo function for testing FFI (returns value * 2)
     * @param value input value
     * @return value * 2
     * @throws OmnisystemException if kernel not initialized
     */
    public int echoInt(int value) throws OmnisystemException {
        checkInitialized();
        return nativeEchoInt(value);
    }

    /**
     * Get comprehensive system statistics
     * @return SystemStats object
     * @throws OmnisystemException if kernel not initialized
     */
    public SystemStats getStats() throws OmnisystemException {
        checkInitialized();
        return new SystemStats(
                getTotalMemory(),
                getAllocatedMemory(),
                getFreeMemory(),
                getProcessCount(),
                getHealth()
        );
    }

    /**
     * Gracefully shutdown Omnisystem
     * @throws OmnisystemException if shutdown fails
     */
    public void shutdown() throws OmnisystemException {
        checkInitialized();
        int result = nativeShutdown();
        if (result == 0) {
            initialized = false;
            System.out.println("✓ Omnisystem shutdown complete\n");
        } else {
            throw new OmnisystemException(String.format("Shutdown failed (error code: %d)", result));
        }
    }

    /**
     * Check if kernel is initialized
     * @return true if initialized
     */
    public boolean isInitialized() {
        return initialized;
    }

    /**
     * Internal: verify kernel is initialized
     * @throws OmnisystemException if not initialized
     */
    private void checkInitialized() throws OmnisystemException {
        if (!initialized) {
            throw new OmnisystemException("Kernel not initialized. Call initialize() first.");
        }
    }

    /**
     * System statistics container
     */
    public static class SystemStats {
        public final double totalMemoryMB;
        public final double allocatedMemoryMB;
        public final double freeMemoryMB;
        public final int processCount;
        public final String health;

        private SystemStats(long totalBytes, long allocatedBytes, long freeBytes, int processCount, String health) {
            this.totalMemoryMB = totalBytes / (1024.0 * 1024.0);
            this.allocatedMemoryMB = allocatedBytes / (1024.0 * 1024.0);
            this.freeMemoryMB = freeBytes / (1024.0 * 1024.0);
            this.processCount = processCount;
            this.health = health;
        }

        @Override
        public String toString() {
            return String.format(
                    "SystemStats{total=%.2f MB, allocated=%.2f MB, free=%.2f MB, processes=%d, health=%s}",
                    totalMemoryMB, allocatedMemoryMB, freeMemoryMB, processCount, health
            );
        }
    }

    /**
     * Exception thrown by Omnisystem operations
     */
    public static class OmnisystemException extends Exception {
        public OmnisystemException(String message) {
            super(message);
        }

        public OmnisystemException(String message, Throwable cause) {
            super(message, cause);
        }
    }

    /**
     * Demo application
     */
    public static void main(String[] args) {
        System.out.println("\n╔═══════════════════════════════════════════════════════════════╗");
        System.out.println("║    OMNISYSTEM JAVA/JNI BINDINGS - POLYGLOT DEMO              ║");
        System.out.println("╚═══════════════════════════════════════════════════════════════╝\n");

        try {
            // Initialize
            Omnisystem omni = new Omnisystem();
            omni.initialize();

            // Get stats
            System.out.println("📊 System Statistics:");
            SystemStats stats = omni.getStats();
            System.out.println("   Total Memory:     " + String.format("%.2f", stats.totalMemoryMB) + " MB");
            System.out.println("   Allocated:        " + String.format("%.2f", stats.allocatedMemoryMB) + " MB");
            System.out.println("   Free:             " + String.format("%.2f", stats.freeMemoryMB) + " MB");
            System.out.println("   Process Count:    " + stats.processCount);
            System.out.println("   Health:           " + stats.health);
            System.out.println();

            // Create processes
            System.out.println("🔧 Creating processes from Java:");
            for (int i = 0; i < 3; i++) {
                long pid = omni.createProcess();
                System.out.println("   ✓ Created process with PID: " + pid);
            }
            System.out.println();

            // Test FFI
            System.out.println("🔍 Testing FFI communication:");
            int testValue = 21;
            int result = omni.echoInt(testValue);
            System.out.println("   echoInt(" + testValue + ") = " + result);
            if (result == testValue * 2) {
                System.out.println("   ✓ FFI working correctly\n");
            }

            // Final stats
            System.out.println("📈 Final Statistics:");
            SystemStats finalStats = omni.getStats();
            System.out.println("   Process count: " + finalStats.processCount);
            System.out.println("   Health:        " + finalStats.health);
            System.out.println();

            // Shutdown
            omni.shutdown();

            System.out.println("╔═══════════════════════════════════════════════════════════════╗");
            System.out.println("║          JAVA/JNI BINDINGS DEMO COMPLETE                     ║");
            System.out.println("║     Java ↔ JNI ↔ C FFI ↔ Rust Kernel WORKING               ║");
            System.out.println("╚═══════════════════════════════════════════════════════════════╝");

        } catch (OmnisystemException e) {
            System.err.println("❌ Omnisystem error: " + e.getMessage());
            e.printStackTrace();
            System.exit(1);
        }
    }
}
