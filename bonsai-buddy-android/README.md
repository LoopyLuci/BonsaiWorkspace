# Bonsai Buddy Android

Sprint 1 scaffold for Bonsai Buddy mobile companion.

## Included in this scaffold

- Jetpack Compose + Material 3 app shell
- Adaptive navigation (bottom nav for compact, rail for medium/expanded)
- Basic chat UI with local persistence
- Networking layer using Ktor (Buddy API + Workspace API hooks)
- Secure token and host storage via EncryptedSharedPreferences
- Room database for cached chat history
- Hilt dependency injection setup
- NSD (mDNS) discovery manager stub

## Package layout

- app/src/main/java/ai/bonsai/buddy/data/network: API client and DTOs
- app/src/main/java/ai/bonsai/buddy/data/storage: secure config/token storage
- app/src/main/java/ai/bonsai/buddy/data/discovery: mDNS discovery
- app/src/main/java/ai/bonsai/buddy/data/db: Room entities/DAO/database
- app/src/main/java/ai/bonsai/buddy/data/repository: chat repository
- app/src/main/java/ai/bonsai/buddy/ui: adaptive app shell and chat screen

## Run

1. Open this folder in Android Studio.
2. Let Android Studio install the requested Android SDK packages.
3. Add a local server host and desktop token via setup UI in Sprint 2 (current values can be seeded in SecureConfigStore during development).
4. Build and run on emulator or device.

## Next sprint targets

- Full setup/discovery screen with QR token onboarding
- SSE streaming chat updates
- Markdown rendering for assistant messages
- Tools catalog and execution flow
