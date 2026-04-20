# Spatial Code Canvas Suggestions

## Executive Summary

The design in Spatial_Code_Canvas.md is strong and implementable in this repo, especially because it aligns with the existing Svelte + Tauri architecture and avoids heavyweight rendering dependencies. The major risks are not in the core idea; they are in operational correctness: Monaco lifecycle bounds, safe persistence/recovery, file edit conflict handling, and deterministic interaction mode state. These need to be addressed as first-class implementation constraints before expanding UI scope.

Current recommendation: proceed, but with reliability gates added early (store + persistence + lifecycle), then layer visual features after those foundations are stable.

## What Is Strong

- The proposed component split is clean and maps well to Bonsai conventions under src/lib/components and src/lib/stores.
- Cursor-anchored zoom math and scale-aware drag are appropriate for transform-based worlds.
- Semantic zoom tiers are a practical approach to avoid always-on Monaco cost.
- Local workspace persistence to .bonsai/canvas.json is the right portability model for this app.
- Keyboard-first controls align with existing developer workflows.
- The implementation order already emphasizes early Rust validation, which is good.

## Critical Gaps

- Unbounded Monaco mounts risk:
	- The design calls out semantic zoom but does not define a hard cap for active editors.
	- Without a mount budget and deterministic dispose policy, memory and responsiveness will degrade quickly.

- Corruption recovery missing:
	- load_canvas_layout returns '{}' when file is absent, but behavior for malformed/corrupt JSON is not specified.
	- Without backup/recovery strategy, one bad write can wipe user layout history.

- Conflict UX for file edits not specified:
	- Same file opened in multiple canvas nodes, or edited outside the canvas, can cause silent overwrite.
	- A stale/dirty conflict indicator and reconciliation path are required.

- Interaction mode collisions:
	- select, pan, connect, note, text editing, and resize can overlap unless there is an explicit state machine.
	- Without strict mode arbitration, pointer interactions become unpredictable.

- Performance acceptance criteria absent:
	- No defined targets for max nodes/connections, frame latency, or load times.
	- No stress verification for minimap + connections + zoom simultaneously.

## Recommendations (High Impact)

- Enforce Monaco lifecycle constraints from day one:
	- Add a global mount budget for canvas editors.
	- Dispose editors immediately when leaving interactive zoom tier or when nodes are offscreen.
	- Keep only lightweight preview text at low zoom.

- Harden persistence and recovery:
	- Add schema_version and saved_at to layout JSON.
	- On load parse failure, move corrupt file to canvas.corrupt.<timestamp>.json and return safe defaults.
	- Use atomic write (temp file + rename) to reduce partial write risk.

- Add conflict detection UX:
	- Track file revision/timestamp per file node.
	- If disk content changed since node load, mark node as conflict and block silent autosave.
	- Offer explicit actions: reload from disk, overwrite disk, or copy to note/chat node.

- Implement explicit mode state machine:
	- Single source of truth for current interaction mode and active gesture.
	- Pointer-down only valid for one mode at a time.
	- Escape always cancels transient actions (connect drag, marquee, resize).

- Add measurable performance gates:
	- Define targets (for example: smooth interaction at 40 nodes / 80 connections).
	- Throttle expensive updates to requestAnimationFrame.
	- Culling/virtualization for offscreen nodes and non-visible connection paths.

## Revised Build Order

1. Canvas store and type schema with explicit mode machine and conflict fields.
2. Rust save/load commands with atomic persistence and corruption recovery.
3. Register commands and run cargo check.
4. Core canvas shell (world viewport, pan/zoom, selection).
5. Node shell (drag/resize/focus) without Monaco.
6. Connection layer and deterministic connect/cancel behavior.
7. Minimap and viewport sync.
8. File nodes with Monaco mount budget + dispose policy.
9. Sticky and chat nodes.
10. Search palette and keyboard shortcuts.
11. App integration toggle and layout coexistence.
12. Stress validation and build/test gate.

This order intentionally moves Monaco-heavy work after core interaction correctness so lifecycle constraints can be validated in context.

## Verification Checklist

- Save/load roundtrip preserves nodes, viewport, and connections.
- Corrupt canvas.json is recovered automatically and defaults load safely.
- At low zoom, file nodes render preview only (no active editor mounts).
- Editor mount count remains within configured budget under heavy canvases.
- External file change triggers visible conflict state in relevant file nodes.
- Conflict actions (reload/overwrite) are explicit and deterministic.
- Escape cancels temporary operations in every mode.
- Connection create/delete remains stable during pan/zoom.
- Minimap remains accurate under large world extents.
- Build passes (frontend + Rust) after integration.

## Go / No-Go

Go:

- If mount budget/dispose lifecycle is implemented and verified.
- If corruption recovery and atomic persistence are live.
- If conflict UX exists and prevents silent data loss.
- If interaction mode arbitration is deterministic under stress.

No-Go:

- If Monaco instances are unbounded.
- If malformed persistence can break startup without recovery.
- If file conflicts can silently overwrite user work.
- If pointer interactions conflict across modes and create nondeterministic behavior.

Bottom line: the canvas concept is strong; ship only after reliability controls are built in, not bolted on.

