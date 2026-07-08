package ai.omnisystem.shared;

oneway interface IOmnisystemCallback {
    void onToken(String token);
    void onComplete();
    void onError(String error);
}
