# Bonsai Everywhere Browser Extension

Cross-browser extension for local Bonsai services.

## Features (current)

- Popup quick actions for connection checks and page summarization
- Sidebar chat with streaming Buddy responses
- Content script page snapshot extraction
- Action confirmation overlay before automation
- Local-only audit log in extension storage
- Omnibox and context-menu integrations

## Build

```bash
cd browser-extension
npm install
npm run build:chrome
```

Load `browser-extension/dist` as an unpacked extension in Chromium browsers.

### Firefox

```bash
npm run build:firefox
```

Load `browser-extension/dist` as a temporary add-on in Firefox Developer Edition.

## Security Notes

- API hosts are constrained to localhost defaults.
- Authentication token is stored in extension local storage.
- Every automated action requires explicit user approval.
- Audit log remains local in extension storage.
