package ai.bonsai.shared;

import ai.bonsai.shared.IBonsaiCallback;

interface IBonsaiService {
    // Legacy LLM interface (deprecated in favor of nativeChat*)
    long initModel(String modelPath, String tokenizerPath);
    String chat(long handle, String prompt, float temperature);
    void generateStream(long handle, String prompt, IBonsaiCallback callback);

    // New JNI-based LLM interface (Phase 2-3)
    String nativeInitModel(String modelPath);
    String nativeChat(String modelId, String messagesJson, float temperature, int maxTokens);
    void nativeChatStream(String modelId, String messagesJson, float temperature, IBonsaiCallback callback);
    boolean nativeUnloadModel(String modelId);
    List<String> nativeGetAvailableModels();
    String nativeGetSessionInfo(String sessionId);

    // Token & Transfer management
    boolean loadToken(in byte[] token);
    boolean verifyToken(String peerId);
    long startTransferDaemon(String configPath);
    long connectToPeer(String peerId, in byte[] token);
    void injectInput(long sessionHandle, int eventType, in byte[] data);
    void releaseHandle(long handle);
    void shutdown();
}
