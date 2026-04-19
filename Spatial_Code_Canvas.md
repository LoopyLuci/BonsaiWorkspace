Bonsai Workspace: Spatial Code Canvas
Context
The user wants an infinite spatial code canvas — inspired by codecanvas.app — that gives coders maximum spatial power: pan/zoom across multiple files simultaneously, draw connections between them, add sticky notes, view live Monaco editors per-file, and navigate via a minimap. All persistence is local (Tauri backend writes .bonsai/canvas.json per workspace). No WebGL or third-party canvas library is added — the canvas is implemented entirely in Svelte using CSS transforms for pan/zoom, SVG for connections, and lightweight Monaco instances per file node.

Architecture Overview
CodeCanvas.svelte              ← full-screen overlay, mounted in App.svelte
├── canvas-world (div)         ← CSS transform: translate(vp.x,vp.y) scale(vp.scale)
│   ├── CanvasGrid.svelte      ← dotted grid background (pure CSS, no JS)
│   ├── CanvasNode.svelte × N  ← draggable / resizable shell per node
│   │   ├── CanvasFileNode     ← Monaco editor (read + edit, lightweight)
│   │   ├── CanvasStickyNote   ← contenteditable colored card
│   │   └── CanvasChatNode     ← frozen AI response snippet
│   └── CanvasConnectionLayer  ← SVG overlay, bezier curves between nodes
├── CanvasMinimap.svelte        ← fixed 180×120 px thumbnail + viewport rect
├── CanvasToolbar.svelte        ← fixed top-left: select / pan / connect / note tools
└── CanvasNodeSearch.svelte     ← Cmd+K palette to add files to canvas
Pan / Zoom (zero dependencies)
<div class="canvas-world"
  style="transform: translate({$viewport.x}px,{$viewport.y}px) scale({$viewport.scale});
         transform-origin: 0 0">
Pan: pointerdown on blank canvas → capture → pointermove accumulates Δx, Δy
Zoom: wheel event → adjust scale then reanchor x/y so cursor stays fixed:
const f = e.deltaY < 0 ? 1.1 : 0.9;
vp = { scale: clamp(vp.scale * f, 0.08, 4),
       x: vp.x + (cursorX - vp.x) * (1 - f),
       y: vp.y + (cursorY - vp.y) * (1 - f) };
Node drag: pointerdown on node header → on pointermove: node.x += Δx / scale
SVG Connection Layer
<svg style="position:absolute;inset:0;pointer-events:none;overflow:visible">

Each connection is a cubic bezier:

M x1 y1 C (x1+120) y1 (x2-120) y2 x2 y2
where (x1,y1) = right-center of source node, (x2,y2) = left-center of target node.

Drawing mode: Select "Connect" tool → hover shows port circles → drag from port → rubber-band line follows cursor → drop on another node's port → connection committed.

Semantic Zoom Levels
Scale	Node appearance
< 0.25	Header stripe + label only (40px tall, collapsed view)
0.25 – 0.6	Header + first 6 lines of text (non-interactive, gray)
> 0.6	Full live Monaco editor, interactive
New Files
File	Purpose
src/lib/stores/canvas.ts	All canvas state: nodes, connections, viewport, tool mode
src/lib/components/CodeCanvas.svelte	Main orchestrator + pan/zoom event handling
src/lib/components/canvas/CanvasGrid.svelte	CSS dotted grid (moves with viewport)
src/lib/components/canvas/CanvasNode.svelte	Draggable/resizable shell
src/lib/components/canvas/CanvasFileNode.svelte	Monaco inside a node
src/lib/components/canvas/CanvasStickyNote.svelte	Editable colored note
src/lib/components/canvas/CanvasChatNode.svelte	Frozen AI snippet
src/lib/components/canvas/CanvasConnectionLayer.svelte	SVG bezier connections
src/lib/components/canvas/CanvasMinimap.svelte	Minimap with viewport rect
src/lib/components/canvas/CanvasToolbar.svelte	Tool selector
src/lib/components/canvas/CanvasNodeSearch.svelte	Cmd+K file-add palette
Modified Files
File	Change
src/App.svelte	"Canvas" toolbar button; mount CodeCanvas as full-screen overlay; pass theme
src-tauri/src/commands.rs	Add save_canvas_layout, load_canvas_layout
src-tauri/src/lib.rs	Register 2 new commands in invoke_handler!
Store Design (src/lib/stores/canvas.ts)
type NodeType  = 'file' | 'sticky' | 'chat';
type ToolMode  = 'select' | 'pan' | 'connect' | 'note';

interface CanvasNode {
  id: string;
  type: NodeType;
  x: number; y: number;
  width: number; height: number;
  z_index: number;
  collapsed: boolean;
  color: string;           // header stripe color
  label: string;
  // type payloads:
  file_path?: string;      // file nodes
  note_text?: string;      // sticky notes
  chat_content?: string;   // chat nodes
}

interface CanvasConnection {
  id: string; from_id: string; to_id: string; label?: string;
}

interface CanvasViewport { x: number; y: number; scale: number; }

// Stores
export const nodes              = writable<CanvasNode[]>([]);
export const connections        = writable<CanvasConnection[]>([]);
export const viewport           = writable<CanvasViewport>({ x: 80, y: 80, scale: 1 });
export const selectedIds        = writable<Set<string>>(new Set());
export const toolMode           = writable<ToolMode>('select');
export const pendingConnection  = writable<{ from_id: string; x: number; y: number } | null>(null);

// Derived
export const maxZ = derived(nodes, $n => Math.max(0, ...$n.map(n => n.z_index)));

// Actions
export function addFileNode(filePath: string, x: number, y: number): void
export function addStickyNote(x: number, y: number): void
export function addChatNode(content: string, x: number, y: number): void
export function updateNode(id: string, patch: Partial<CanvasNode>): void
export function deleteNode(id: string): void
export function addConnection(fromId: string, toId: string): void
export function deleteConnection(id: string): void
export function bringToFront(id: string): void
export function fitView(canvasEl: HTMLElement): void  // auto-fit all nodes in viewport
export async function saveCanvas(workspacePath: string): Promise<void>
export async function loadCanvas(workspacePath: string): Promise<void>
Rust Commands
#[tauri::command]
pub async fn save_canvas_layout(workspace_path: String, layout_json: String) -> Result<(), String>
// → creates <workspace_path>/.bonsai/ dir if needed, writes canvas.json

#[tauri::command]
pub async fn load_canvas_layout(workspace_path: String) -> Result<String, String>
// → reads <workspace_path>/.bonsai/canvas.json, returns "{}" if absent
Keyboard Shortcuts
Key	Action
V	Select tool
H	Pan tool
C	Connect tool
N	Add sticky note at viewport center
Cmd+K	Node search palette (add file to canvas)
Backspace / Delete	Delete selected nodes
Escape	Cancel pending connection / deselect all
Cmd+0	Fit all nodes in view
Cmd+= / Cmd+-	Zoom in / out
Cmd+S	Save canvas layout
CanvasFileNode Monaco Details
createEditor() from existing $lib/utils/monaco.ts — reuses same factory
Options override: minimap:false, lineNumbers:'on', fontSize:12, readOnly:false, padding:{top:8}
Language: setLanguageFromPath(editor, node.file_path) — reuses existing util
Load: invoke('read_file', {path: node.file_path}) on mount
Auto-save: 1s debounce invoke('write_file', ...) on content change
Monaco is only onMounted when scale > 0.6; below that threshold a static <pre> preview is shown instead (avoids spawning dozens of heavy editors at low zoom)
Minimap Detail
Fixed 180×120px overlay, position:fixed, bottom-right corner
Renders each node as a <rect> in a <svg viewBox> computed to fit all nodes
Viewport rectangle shown as a white dashed border rect scaled to match
Click on minimap → centers viewport on world position
Drag on minimap → pans
Implementation Order
Step	File	Action
1	src/lib/stores/canvas.ts	Full store with types and all action functions
2	src-tauri/src/commands.rs	save_canvas_layout + load_canvas_layout
3	src-tauri/src/lib.rs	Register 2 commands
4	cargo check	Validate Rust
5	src/lib/components/canvas/CanvasNode.svelte	Draggable/resizable shell
6	src/lib/components/canvas/CanvasFileNode.svelte	Monaco in node
7	src/lib/components/canvas/CanvasStickyNote.svelte	Editable note
8	src/lib/components/canvas/CanvasChatNode.svelte	AI snippet
9	src/lib/components/canvas/CanvasGrid.svelte	CSS dotted grid
10	src/lib/components/canvas/CanvasConnectionLayer.svelte	SVG bezier connections
11	src/lib/components/canvas/CanvasMinimap.svelte	Minimap
12	src/lib/components/canvas/CanvasToolbar.svelte	Tool selector
13	src/lib/components/canvas/CanvasNodeSearch.svelte	Cmd+K palette
14	src/lib/components/CodeCanvas.svelte	Main orchestrator
15	src/App.svelte	Wire "Canvas" button, mount overlay
16	npm run build	Validate frontend
Verification
cargo check passes after step 4
npm run build passes after step 16
Canvas toolbar button opens full-screen canvas over editor
Blank dotted grid renders; pan and zoom work (cursor-anchored zoom)
Cmd+K opens palette → type filename → Enter → Monaco node appears at viewport center
Drag node by header → repositions correctly at all zoom levels
Zoom to < 0.25 → nodes collapse to stripe+label; zoom back → Monaco re-activates
Select Connect tool → drag between two nodes → bezier curve appears
Add sticky note (N key) → type → text persists
Minimap shows all nodes; clicking pans viewport
Cmd+S → .bonsai/canvas.json written to workspace; reload → layout restored
Closing canvas (Escape/button) → normal editor layout unaffected
CSS Design Tokens (appended to existing :root)
--canvas-bg:           #0f0f13;
--canvas-grid:         rgba(255,255,255,0.04);
--canvas-node-bg:      #1a1a22;
--canvas-node-border:  #2e2e3a;
--canvas-node-shadow:  0 4px 24px rgba(0,0,0,0.5);
--canvas-connection:   rgba(74,158,255,0.6);
--canvas-port:         #4a9eff;
--canvas-select-ring:  rgba(74,158,255,0.25);