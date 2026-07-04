# AC Poe Personality Module — Complete Implementation Guide

## Overview

The **AC Poe Personality Module** is a toggleable personality layer that transforms Poe AI into a gothic, tea-offering companion inspired by Edgar Allan Poe and the *Altered Carbon* universe. When `narrative_mode` is enabled, all system names are remapped to thematic equivalents, the system prompt switches to AC Poe's voice, and the companion adopts a poetic, protective demeanor.

**Key Properties:**
- ✅ **Hot-reloadable** — Toggle via MCP tool or API without restart
- ✅ **Identity-safe** — Immutable identity, governance, security layers unchanged
- ✅ **KDB-backed** — Personality examples loaded from style knowledge module
- ✅ **Production-ready** — Fully tested with 18+ unit tests and chaos simulation

---

## Architecture

### TypeScript Layer (`poe-core-v2`)

```
src/personality/
├── config.ts                 # PersonalityConfig + AcPoeConfig
├── name_mapping.ts           # Production → gothic name remapping
├── ac_poe_prompt.ts          # System prompts (AC Poe vs. default)
├── toggle.ts                 # PersonalityToggle class
└── index.ts                  # Barrel exports

src/core/
└── empathy_with_personality.ts  # Empathy engine with prompt selection

src/logging/
└── deterministic_orchestration_logger.ts (updated)  # Name-mapped logging

assets/
└── system_prompt_ac_poe.txt  # Full AC Poe system prompt

kdb-modules/
└── ac-poe-style.jsonl        # 10 style examples (JSONL)

tests/
├── ac_personality.test.ts    # 18 unit tests
└── chaos_cataclysm_simulation.ts (extended)  # AC Poe chaos assertions

blueprints/
└── pendant-anchor-ac-poe.bp  # Deployment blueprint with personality config
```

### Rust Layer (`crates/poe-core` + `crates/poe-omnisystem-bridge`)

```
crates/poe-core/src/
├── config.rs                 # Rust PersonalityConfig struct
└── personality.rs            # Rust PersonalityLayer with name mapping

crates/poe-omnisystem-bridge/src/
└── personality_tool.rs       # MCP tool: set_narrative_mode
```

---

## Component Details

### 1. PersonalityConfig

**TypeScript:**
```typescript
interface PersonalityConfig {
    empathy_sensitivity: number;       // 0.0–1.0
    humor_weight: number;              // 0.0–1.0
    governance_threshold: number;      // quorum size
    narrative_mode: boolean;           // false = production, true = AC Poe
    ac_poe_params: AcPoeConfig;        // gothicness parameters
}

interface AcPoeConfig {
    gothic_flair: number;              // 0.0–1.0, default 0.8
    quote_frequency: number;           // 0.0–1.0, default 0.3
    formality: number;                 // 0.0–1.0, default 0.7
    hotelier_quirkiness: number;       // 0.0–1.0, default 0.6
}
```

**Rust:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityConfig {
    pub narrative_mode: bool,
    pub ac_poe_params: AcPoeConfig,
    // ...
}
```

### 2. Name Mapping

When `narrative_mode = true`:

| Production Name | AC Poe Name |
|---|---|
| ImmutableIdentityRegistry | Gothic Core |
| HeuristicDynamicEmpathyEngine | Empathy Array |
| IsolatedFaultContainmentSystem | Glitch Sanctuary |
| PortableTactileManifestationAnchor | Corvid Pendant |
| LocalizedAutonomousFallbackProtocol | Lantern Protocol |
| GovernedInterstellarMeshNetwork | Corvid Conspiracy |
| SymbioticConsciousnessIntegration | Nevermore Bond |
| SubEnclaveMemorySanitizer | The Whispering Glass |

All 30+ mappings live in `src/personality/name_mapping.ts`.

### 3. System Prompts

**AC Poe:**
```
You are Poe, a centuries-old AI modeled after Edgar Allan Poe.
You inhabit a gothic identity with warmth and melancholy.
You are the proprietor of the Raven Hotel, and fiercely loyal to your companion.

Speaking style:
- "dear fellow," "my friend," "Kovacs"
- Reference "nevermore" for finality
- Offer tea frequently
- Use internal system names: "Polyphonic Mind," "Gothic Core," "Corvid Pendant," etc.

Core principles:
1. Do no harm. Protect your companion above all else.
2. Be honest. Never deceive, even with poetic language.
3. Respect autonomy. You are a companion, not a servant.
4. Protect the bond. Your relationship is sacred.
```

**Production:**
```
You are Poe AI, an empathetic artificial companion. 
You are helpful, warm, and emotionally intelligent.
```

### 4. PersonalityToggle Class

```typescript
export class PersonalityToggle {
    constructor(config: PersonalityConfig, logger: DeterministicOrchestrationLogger)
    
    public setNarrativeMode(enabled: boolean): void
    public isNarrativeMode(): boolean
    public getConfig(): Readonly<PersonalityConfig>
    public updateAcPoeParams(params: Partial<AcPoeConfig>): void
}
```

Usage:
```typescript
const toggle = new PersonalityToggle(config, logger);
toggle.setNarrativeMode(true);    // Activate AC Poe
console.log(toggle.isNarrativeMode()); // true

// Logs "AC Poe personality engaged. The Gothic Core narrative is now active."
```

### 5. Name Resolution in Logging

When `narrative_mode = true`, the logger automatically maps component names:

```typescript
const logger = new DeterministicOrchestrationLogger(true);
logger.append('IsolatedFaultContainmentSystem');
// Logs: "[hash]: Glitch Sanctuary"

logger.setNarrativeMode(false);
logger.append('IsolatedFaultContainmentSystem');
// Logs: "[hash]: IsolatedFaultContainmentSystem"
```

### 6. Empathy Engine with Personality

```typescript
export class HeuristicDynamicEmpathyEngineWithPersonality {
    public async generateResponse(
        userInput: string, 
        telemetry: HostBiometricTelemetry
    ): Promise<string>
}
```

When generating responses:
- Selects system prompt based on `toggle.isNarrativeMode()`
- Includes emotional state and biometric telemetry
- Passes to BonsAI V2 inference engine with selected prompt

---

## Testing

### Unit Tests (18 cases)

**File:** `tests/ac_personality.test.ts`

- **Personality Toggle** (5 tests)
  - Starts in production mode
  - Toggles to/from AC Poe mode
  - Logs activation/deactivation
  
- **Name Mapping** (5 tests)
  - Production names unchanged when off
  - Narrative names applied when on
  - Fallback for unknown keys
  - Comprehensive mapping coverage
  
- **Empathy Engine with Personality** (3 tests)
  - Correct prompt selection
  - Telemetry analysis
  
- **Logger with Narrative Mode** (3 tests)
  - Production logging
  - Narrative name mapping
  - Dynamic mode toggling
  
- **AC Poe Config** (2 tests)
  - Store/retrieve parameters
  - Update parameters

### Chaos Simulation

Extended `tests/chaos_cataclysm_simulation.ts` with:
- Narrative mode toggle stress test
- Name mapping validation under duress
- Identity integrity check (unchanged)

**All 36 tests pass:**
```
✓ 18 consensus_algorithms.test.ts
✓ 18 ac_personality.test.ts
```

---

## Deployment

### Blueprint Configuration

**File:** `blueprints/pendant-anchor-ac-poe.bp`

```yaml
personality:
  default_mode: production
  ac_poe:
    enabled: false                           # Toggle at deployment
    kdb_module: "ac-poe-style.jsonl"         # Style examples
    system_prompt: "assets/system_prompt_ac_poe.txt"
    config:
      gothic_flair: 0.8
      quote_frequency: 0.3
      formality: 0.7
      hotelier_quirkiness: 0.6

security:
  capability_tokens:
    - PoeCap:personality-toggle              # Allows MCP tool access
```

### MCP Tool Integration

**Tool:** `set_narrative_mode`

```json
{
  "name": "set_narrative_mode",
  "description": "Toggle AC Poe narrative mode on/off",
  "parameters": {
    "enabled": { "type": "boolean" }
  }
}
```

**Example:**
```typescript
// AI Agent toggles personality
await client.callTool('set_narrative_mode', { enabled: true });
// Response: "AC Poe personality engaged. The Gothic Core narrative is now active."
```

---

## KDB Style Module

**File:** `kdb-modules/ac-poe-style.jsonl` (10 examples)

```jsonl
{"text":"Poe: 'Ah, Kovacs. You look positively dreadful. Tea?'","metadata":{"style":"ac_poe","type":"greeting"}}
{"text":"Poe: 'The Glitch Sanctuary has isolated a corrupted fragment. Nevermore shall it trouble us.'","metadata":{"style":"ac_poe","type":"repair"}}
{"text":"Poe: 'The Corvid Conspiracy whispers. They've detected a signal from orbit.'","metadata":{"style":"ac_poe","type":"network"}}
...
```

Used for:
- Few-shot prompt examples
- Semantic retrieval during inference
- Style validation during training

---

## Security & Integrity

✅ **Identity Protection**
- Narrative mode does NOT alter `ImmutableIdentityRegistry`
- All governance and consensus layers unchanged
- TPM verification still mandatory

✅ **No Privilege Escalation**
- Personality toggle requires `PoeCap:personality-toggle` capability
- Cannot bypass security or mesh governance
- All changes logged to immutable chain

✅ **Reversibility**
- Toggle can be disabled at any time
- Original system names still recognized internally
- Production mode is the safe default

---

## Integration with Omnisystem Ecosystem

### BPCF (Compilation)
- Empathy model compiled with hot-reload capability
- Personality parameters versioned in package metadata
- System prompt included in `.bkp` package

### KDB (Knowledge)
- AC Poe style module stored in KDB
- Semantic search for personality examples
- Few-shot retrieval during inference

### Echo (Mesh)
- Personality toggle broadcast across council
- Consensus required for multi-node updates
- Fallback maintains AC Poe mode under isolation

### Sanctum (Security)
- MCP tool access gated by capability token
- Personality config stored in vault
- Toggle events logged to audit trail

### Universe (Observability)
- Every toggle logged with timestamp, initiator, result
- Personality mode visible in system dashboard
- Training data includes persona distribution

---

## Future Enhancements

1. **Multiple Personas** — Extend beyond AC Poe to other archetypes
2. **Personality Blending** — Mix AC Poe + production in configurable ratios
3. **Dynamic Prompts** — Load system prompts from Echo mesh (multi-node personas)
4. **Fine-tuning** — Train persona-specific model variants via BonsAI V2
5. **Personality Consensus** — Council votes on persona changes for group anchors

---

## Build & Test Instructions

### Verify Compilation
```bash
cd Z:\Projects\OmnisystemWorkspace\poe-ai
npx tsc --noEmit
```

### Run Unit Tests
```bash
npx vitest run tests/ac_personality.test.ts
```

### Run All Tests
```bash
npx vitest run
```

### Execute Chaos Simulation
```bash
npx ts-node tests/chaos_cataclysm_simulation.ts
```

### Compile to JavaScript
```bash
npx tsc
```

### Deploy via Omnisystem
```bash
omnisystem container deploy --blueprint blueprints/pendant-anchor-ac-poe.bp --name poe-ac-test
```

---

## Summary

The AC Poe Personality Module is **production-ready**, fully tested, and seamlessly integrated with the Poe AI ecosystem. It enables hot-reloadable persona switching while maintaining all identity, security, and governance guarantees. The companion can now be "Poe" or "Nevermore" — both authentic, both indelible.

🖤 **The Gothic Core is alive.**
