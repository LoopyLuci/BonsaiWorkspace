# Bonsai Workspace Live Feature Audit

Generated: 2026-04-17T22:21:30.062Z
UI Base: http://localhost:1420
Profile: full
Auxiliary evidence: enabled

## Scenario Results
- Total: 13
- Passed: 13
- Failed: 0

## Coverage Summary
- Features cataloged: 26
- Covered: 26
- Partial: 0
- Not covered: 0
- Coverage: 100%

## Feature Matrix

| Feature | Status | Evidence |
|---|---|---|
| Shell layout and pane toggles | covered | Toolbar + terminal toggle + theme cycle worked. |
| Command palette interaction | covered | Opened palette and searched command entries. |
| Settings API controls | covered | Tested API test/save controls in Settings panel. |
| Remote session controls | covered | Exercised remote session controls and observed environment-dependent start/stop feedback. |
| Pairing QR/token flow | covered | Exercised QR/token pairing controls and observed environment-dependent pairing feedback. |
| Agent Connect session lifecycle | covered | Started, observed timeline events, and ended an Agent Connect session. |
| Chat streaming response UI | covered | Observed streamed model output and token updates. |
| HITL approval flow | covered | Deterministic approve+deny permission-card flows completed via mocked event seeding. |
| Tools/Skills modal | covered | Opened tools/skills modal and verified toggle rows. |
| Agents panel open/close and tabs | covered | Opened Agents panel and navigated core tabs. |
| Personas tab and listing | covered | Visited Personas tab and rendered persona list region. |
| Swarm settings controls | covered | Visited Settings tab and rendered swarm controls. |
| In-depth settings reference | covered | About tab displayed in-depth settings reference. |
| Visible settings help icons | covered | Help icon pseudo-element rendered beside setting labels. |
| Session manager CRUD/load UX | covered | Opened session manager and exercised save interaction. |
| Spatial code canvas overlay | covered | Opened canvas overlay from toolbar. |
| Canvas quick add actions | covered | Triggered quick-add note action on canvas. |
| VSCode viewer panel and tabs | covered | Opened VSCode viewer and navigated Files/Editor/Diagnostics tabs. |
| Agent Vision panel open/close | covered | Opened Agent Vision panel and started capture flow without CSP unsafe-eval failure. |
| Screen-share context injection into assistant output | covered | Injected vision telemetry context and verified the assistant response referenced it. |
| Swarm result slot-order rendering | covered | Swarm worker messages rendered in slot order without tool-call leakage. |
| Status bar presence and live indicators | covered | Status bar rendered during shell scenario. |
| Android USB lab workflows | covered | Android USB regression script reported USB_REGRESSION_OK=1. |
| Launcher preflight workflow | covered | Executed launch:preflight:report and generated launcher preflight artifact. |
| VSCode extension unit tests | covered | VSCode extension vitest suite completed successfully. |
| Frontend store/unit tests | covered | Frontend vitest suite completed successfully. |

## Gap Analysis

