# Bonsai Workspace Live Feature Audit

Generated: 2026-04-17T04:21:47.259Z
UI Base: http://localhost:1420
Profile: smoke
Auxiliary evidence: disabled

## Scenario Results
- Total: 6
- Passed: 6
- Failed: 0

## Coverage Summary
- Features cataloged: 25
- Covered: 14
- Partial: 11
- Not covered: 0
- Coverage: 56%

## Feature Matrix

| Feature | Status | Evidence |
|---|---|---|
| Shell layout and pane toggles | covered | Toolbar + terminal toggle + theme cycle worked. |
| Command palette interaction | covered | Opened palette and searched command entries. |
| Settings API controls | covered | Tested API test/save controls in Settings panel. |
| Remote session controls | covered | Started/stopped remote session and sent test input. |
| Pairing QR/token flow | covered | Exercised QR/token pairing actions. |
| Agent Connect session lifecycle | partial | Skipped in fast smoke profile. Run full evidence profile for complete coverage. |
| Chat streaming response UI | covered | Observed streamed model output and token updates. |
| HITL approval flow | covered | Deterministic approve+deny permission-card flows completed via mocked event seeding. |
| Tools/Skills modal | partial | Skipped in fast smoke profile. Run full evidence profile for complete coverage. |
| Agents panel open/close and tabs | covered | Opened Agents panel and navigated core tabs. |
| Personas tab and listing | covered | Visited Personas tab and rendered persona list region. |
| Swarm settings controls | covered | Visited Settings tab and rendered swarm controls. |
| In-depth settings reference | covered | About tab displayed in-depth settings reference. |
| Visible settings help icons | covered | Help icon pseudo-element rendered beside setting labels. |
| Session manager CRUD/load UX | partial | Skipped in fast smoke profile. Run full evidence profile for complete coverage. |
| Spatial code canvas overlay | partial | Skipped in fast smoke profile. Run full evidence profile for complete coverage. |
| Canvas quick add actions | partial | Skipped in fast smoke profile. Run full evidence profile for complete coverage. |
| VSCode viewer panel and tabs | partial | Skipped in fast smoke profile. Run full evidence profile for complete coverage. |
| Agent Vision panel open/close | partial | Skipped in fast smoke profile. Run full evidence profile for complete coverage. |
| Swarm result slot-order rendering | covered | Swarm worker messages rendered in slot order without tool-call leakage. |
| Status bar presence and live indicators | covered | Status bar rendered during shell scenario. |
| Android USB lab workflows | partial | USB regression script exists; run with BONSAI_AUDIT_RUN_AUX=1 on device-capable machine. |
| Launcher preflight workflow | partial | Launcher and preflight scripts detected; run with BONSAI_AUDIT_RUN_AUX=1 for evidence execution. |
| VSCode extension unit tests | partial | Unit tests present in vscode-extension/src/test; run with BONSAI_AUDIT_RUN_AUX=1 for evidence execution. |
| Frontend store/unit tests | partial | Frontend unit tests present in src/lib stores/utils; run with BONSAI_AUDIT_RUN_AUX=1 for evidence execution. |

## Gap Analysis

- Agent Connect session lifecycle (partial): Add dedicated scenario assertions in live harness for this feature area.
- Tools/Skills modal (partial): Add dedicated scenario assertions in live harness for this feature area.
- Session manager CRUD/load UX (partial): Add dedicated scenario assertions in live harness for this feature area.
- Spatial code canvas overlay (partial): Add dedicated scenario assertions in live harness for this feature area.
- Canvas quick add actions (partial): Add dedicated scenario assertions in live harness for this feature area.
- VSCode viewer panel and tabs (partial): Add dedicated scenario assertions in live harness for this feature area.
- Agent Vision panel open/close (partial): Add dedicated scenario assertions in live harness for this feature area.
- Android USB lab workflows (partial): Run device-in-the-loop Android USB regression on a machine with adb and physical device access.
- Launcher preflight workflow (partial): Execute launcher preflight scripts in an end-to-end desktop launch pipeline and assert report artifacts.
- VSCode extension unit tests (partial): Run extension test suite in CI and export junit/coverage artifacts for audit linkage.
- Frontend store/unit tests (partial): Run vitest suite and include coverage thresholds for critical stores/components.
