### BonsaiBot  — Feature Reference

Everything in this document is purely concept, does not exist yet, and needs to be customized and designed for to work with the Bonsai Workspace and Bonsai Buddy.
---

#### Core Concept
- A **personal AI assistant** you run on your own devices
- Answers on the channels you already use
- **Local-first** architecture — the Gateway is the control plane, the assistant is the product
- Single-user focused: feels local, fast, and always-on

---

#### Multi-Channel Inbox
Supports messaging across a massive range of platforms:

| Category | Platforms |
|----------|-----------|
| Mobile | WhatsApp, Signal, iMessage, BlueBubbles (iMessage), LINE, Zalo, Zalo Personal, WeChat, QQ |
| Desktop/Web | Slack, Discord, Microsoft Teams, Google Chat, Telegram, IRC, Mattermost, Matrix, Nextcloud Talk, Feishu, Nostr, Synology Chat, Tlon, Twitch, WebChat |
| Native | macOS, iOS, Android |

---

#### Gateway (Local-first Control Plane)
- Single control plane for **sessions, channels, tools, and events**
- Runs as a persistent background daemon via `launchd` (macOS) or `systemd` (Linux)
- Configurable port (e.g. `--port 18789`)
- Verbose/debug mode (`--verbose`)
- Remote gateway control over SSH
- WebSocket-based node pairing protocol

---

#### Voice Features
- **Voice Wake** — wake-word detection on macOS and iOS
- **Talk Mode** — continuous voice conversation on Android
- Voice trigger forwarding
- ElevenLabs TTS integration + system TTS fallback
- Push-to-talk overlay (macOS app)

---

#### Live Canvas
- Agent-driven **visual workspace**
- Built with **A2UI** framework
- Renderable from macOS, iOS, and Android
- Canvas surface exposed as a first-class tool

---

#### Multi-Agent Routing
- Route inbound channels, accounts, and peers to **isolated agents**
- Supports workspaces + per-agent sessions
- Each agent can have its own isolated session context

---

#### Agent Workspace & Skills
- Workspace root: `~/.bonsaibot/workspace` (configurable via `agents.defaults.workspace`)
- Injected prompt files: `AGENTS.md`, `SOUL.md`, `TOOLS.md`
- Skills stored at: `~/./workspace/skills/<skill>/SKILL.md`
- Skills registry: **BonsaiHub**
- Supports bundled, managed, and workspace skills
- Onboarding-driven skill setup

---

#### First-Class Tools
Built-in tool categories available to the agent:
- **browser** — web browsing
- **canvas** — Live Canvas rendering
- **nodes** — device node management
- **cron** — scheduled/recurring tasks
- **sessions** — session management
- **Discord/Slack actions** — platform-specific actions

---

#### Security & DM Access Controls
- Inbound DMs treated as **untrusted input** by default
- **DM pairing system**: unknown senders receive a pairing code; bot does not process their message until approved
  - Config key: `dmPolicy="pairing"` (also `channels.discord.dmPolicy`, `channels.slack.dmPolicy`)
- Approve senders: `bonsaibot pairing approve <channel> <code>` (adds to local allowlist)
- Open DMs require explicit opt-in: `dmPolicy="open"` + `"*"` in `allowFrom`
- **Sandbox modes**:
  - Default: tools run on host for `main` session (full access for personal use)
  - Group/channel safety: `agents.defaults.sandbox.mode: "non-main"` runs non-main sessions in **per-session Docker sandboxes**
- Typical sandbox defaults:
  - **Allow**: `bash`, `process`, `read`, `write`, `edit`, `sessions_list`, `sessions_history`, `sessions_send`, `sessions_spawn`
  - **Deny**: `browser`, `canvas`, `nodes`, `cron`, `discord`, `gateway`
- `bonsaibot doctor` command surfaces risky/misconfigured DM policies
- Full security guide available in docs

---

#### Companion Apps

**macOS (BonsaiBot.app)**
- Menu bar control for Gateway and health monitoring
- Voice Wake + push-to-talk overlay
- WebChat + debug tools
- Remote gateway control over SSH
- Note: signed builds required for macOS permissions to persist across rebuilds

**iOS Node**
- Pairs as a node over Gateway WebSocket (device pairing)
- Voice trigger forwarding
- Canvas surface
- Controlled via `BonsaiBot nodes …`

**Android Node**
- Pairs as a WS node via device pairing (`bonsaibot devices ...`)
- Exposes Connect/Chat/Voice tabs
- Canvas, Camera, Screen capture support
- Android device command families

---

#### CLI Commands & Operator Tools

**Core CLI:**
```bash
npm install -g bonsaibot@latest
bonsaibot onboard --install-daemon
bonsaibot gateway --port 18789 --verbose
bonsaibot message send --to +1234567890 --message "Hello"
bonsaibot agent --message "Ship checklist" --thinking high
bonsaibot update --channel stable|beta|dev
bonsaibot doctor
```

**Chat commands (in-session):**
- `/status` — show current status
- `/new` — start a new session
- `/reset` — reset session
- `/compact` — compact session history
- `/think <level>` — set thinking level
- `/verbose on|off` — toggle verbose output
- `/trace on|off` — toggle trace logging
- `/usage off|tokens|full` — control usage display
- `/restart` — restart the gateway
- `/activation mention|always` — set activation mode

**Session Tools:**
- `sessions_list`
- `sessions_history`
- `sessions_send`

---

#### Onboarding System
- Interactive CLI onboarding: `bonsaibot onboard`
- Guides through: gateway setup, workspace, channels, and skills
- Works on **macOS, Linux, and Windows (via WSL2 — strongly recommended)**
- Compatible with `npm`, `pnpm`, and `bun`

---

#### Model & Auth Support
- Supports many providers and models
- Recommended: use a current flagship model from your trusted provider
- **OAuth subscriptions**: OpenAI (ChatGPT/Codex)
- Auth profile rotation + fallbacks via **Model Failover**
- Config + CLI model management via **Models** docs

---

#### Configuration
- Config file: 'bonsaibot/bonsaibot.json`
- Minimal config:
```json
{
  "agent": {
    "model": "<provider>/<model-id>"
  }
}
```
- Full configuration reference available in docs

---

#### Development Channels
| Channel | Description |
|---------|-------------|
| **stable** | Tagged releases (`vYYYY.M.D`), npm dist-tag `latest` |
| **beta** | Prerelease tags (`vYYYY.M.D-beta.N`), npm dist-tag `beta` |
| **dev** | Moving head of `main`, npm dist-tag `dev` |

Switch with: `bonsaibot update --channel stable|beta|dev`

---

#### Building from Source
```bash
git clone https://github.com/bonsaibot/bonsaibot.git
cd bonsaibot
pnpm install
pnpm ui:build   # auto-installs UI deps on first run
pnpm build
pnpm bonsaibot onboard --install-daemon
pnpm gateway:watch  # dev loop with auto-reload
```
- `pnpm bonsaibot ...` runs TypeScript directly via `tsx`
- `pnpm build` produces `dist/` for the packaged bonsaibot` binary
- Prefers `pnpm`; Bun is optional for running TypeScript directly