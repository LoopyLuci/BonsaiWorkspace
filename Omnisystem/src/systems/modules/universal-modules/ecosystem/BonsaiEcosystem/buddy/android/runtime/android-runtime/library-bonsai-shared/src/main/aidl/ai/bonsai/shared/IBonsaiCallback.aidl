package ai.bonsai.shared;

oneway interface IBonsaiCallback {
    void onToken(String token);
    void onComplete();
    void onError(String error);
}
