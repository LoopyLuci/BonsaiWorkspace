# Context Skill Clarification Design

## Goal

Prevent wrong tool routing on ambiguous prompts by forcing a quick, structured clarification when intent confidence is low.

## Trigger Conditions

Use clarification mode when any of these are true:

- Prompt is short and ambiguous (for example: "hello", "help", "read this").
- Multiple intents score similarly (within confidence delta threshold).
- Prompt references a file/action without enough arguments.

## UX Contract

Show a compact popup in chat with numbered choices plus freeform fallback:

1. Read or inspect files
2. Run command / diagnostics
3. Explain code or architecture
4. Edit code
5. Other (type your own)

Rules:

- One click should immediately continue execution with selected intent.
- Choosing 5 opens a freeform single-line input.
- Keep the original user text visible as context at top of popup.
- Auto-dismiss popup after selection and append a structured clarification message.

## Message Protocol

Frontend sends this payload when user selects a clarification option:

```json
{
  "type": "context_clarification",
  "original_prompt": "hello",
  "selected_option": 3,
  "selected_label": "Explain code or architecture",
  "freeform": null
}
```

If option 5 is chosen:

```json
{
  "type": "context_clarification",
  "original_prompt": "help",
  "selected_option": 5,
  "selected_label": "Other",
  "freeform": "Summarize how swarm runtime settings change behavior"
}
```

## Backend Behavior

- Add intent classifier stage before tool loop.
- If ambiguity detected, return `action_handled=true` with a `clarification-request` event payload.
- When clarification payload arrives, build a deterministic intent override and proceed.
- Persist clarification decision in session context for follow-up turns.

## Safety + Routing Policy

- Greeting-only intent should never trigger tool calls.
- File operations require explicit file/path intent or explicit user confirmation.
- Command execution remains approval-gated as today.

## Suggested Implementation Order

1. Add event type `clarification-request` from backend for ambiguous prompts.
2. Add popup card component in chat panel for numbered options and freeform input.
3. Add command `resume_after_clarification` that accepts structured payload above.
4. Add telemetry event `chat.clarification_used` with chosen option.

## Acceptance Criteria

- Prompt `Hello` returns conversational response with no tool calls.
- Prompt `read this` always asks clarification before tool invocation.
- Prompt with clear intent skips popup and executes normally.
- Clarification choice is visible in transcript for auditability.
