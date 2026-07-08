"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.generateTauri = generateTauri;
function kindToOwClass(kind, variant) {
    const varSuffix = kind === 'button' ? ` ow-btn-${variant ?? 'primary'}` : '';
    return `ow-${kind === 'button' ? 'btn' : kind}${varSuffix}`;
}
function kindToHtmlTag(kind) {
    const map = {
        button: 'button', input: 'input', textarea: 'textarea', select: 'select',
        label: 'span', form: 'form', navbar: 'nav', list: 'ul', listitem: 'li',
        table: 'table', divider: 'hr', sidebar: 'aside', image: 'img', badge: 'span',
    };
    return map[kind] ?? 'div';
}
function renderHtmlWidget(node) {
    const tag = kindToHtmlTag(node.kind);
    const owClass = kindToOwClass(node.kind, node.variant);
    const idAttr = `id="${node.id ?? node.name?.toLowerCase() ?? 'widget'}"`;
    if (node.kind === 'input') {
        const ph = node.placeholder ? ` placeholder="${node.placeholder}"` : '';
        const val = node.value ? ` value="${node.value}"` : '';
        return `<input ${idAttr} class="${owClass}" type="text"${ph}${val} />`;
    }
    if (node.kind === 'textarea') {
        const ph = node.placeholder ? ` placeholder="${node.placeholder}"` : '';
        return `<textarea ${idAttr} class="${owClass}"${ph}></textarea>`;
    }
    if (node.kind === 'select') {
        const opts = (node.options ?? [{ value: '', label: 'Select...' }])
            .map(o => `    <option value="${o.value}">${o.label}</option>`)
            .join('\n');
        return `<select ${idAttr} class="${owClass}">\n${opts}\n  </select>`;
    }
    if (node.kind === 'toggle') {
        return `<div ${idAttr} class="ow-toggle-wrap" role="switch" aria-checked="false" tabindex="0">\n    <div class="ow-toggle" id="${(node.id ?? 'toggle')}_knob"></div>\n  </div>`;
    }
    if (node.kind === 'checkbox') {
        return `<label class="ow-checkbox">\n    <input ${idAttr} type="checkbox"${node.checked ? ' checked' : ''} />\n    <span class="ow-checkbox-label">${node.label ?? 'Checkbox'}</span>\n  </label>`;
    }
    if (node.kind === 'progress') {
        return `<div ${idAttr} class="ow-progress" role="progressbar" aria-valuenow="0" aria-valuemax="100">\n    <div class="ow-progress-fill" id="${(node.id ?? 'progress')}_fill" style="width:0%"></div>\n  </div>`;
    }
    if (node.kind === 'card') {
        return `<div ${idAttr} class="ow-card">\n    <div class="ow-card-title">${node.label ?? node.name ?? 'Card Title'}</div>\n    <div class="ow-card-body" id="${(node.id ?? 'card')}_body"></div>\n  </div>`;
    }
    if (node.kind === 'modal') {
        return `<div ${idAttr} class="ow-modal" style="display:none">\n    <div class="ow-modal-title">${node.label ?? 'Modal'}</div>\n    <div class="ow-modal-body" id="${(node.id ?? 'modal')}_body"></div>\n    <div class="ow-modal-footer">\n      <button class="ow-btn ow-btn-ghost" id="${(node.id ?? 'modal')}_close">Close</button>\n    </div>\n  </div>`;
    }
    const label = node.label ? node.label : node.name ?? node.kind;
    const disabled = node.disabled ? ' disabled' : '';
    const btnType = node.kind === 'button' ? ' type="button"' : '';
    return `<${tag} ${idAttr} class="${owClass}"${btnType}${disabled}>${label}</${tag}>`;
}
function renderTauriJs(node) {
    const id = node.id ?? node.name?.toLowerCase() ?? 'widget';
    const lines = [];
    lines.push(`const { invoke } = window.__TAURI__.tauri;`);
    lines.push('');
    lines.push(`const widget = document.getElementById('${id}');`);
    lines.push('');
    if (node.kind === 'button') {
        const clickEv = node.events?.find(e => e.name === 'onClick');
        const invokeCmd = clickEv?.handler.replace(/^.*invoke\s*\(\s*['"]([^'"]+)['"].*$/, '$1') ?? `${id}_clicked`;
        lines.push(`widget.addEventListener('click', async () => {`);
        lines.push(`  try {`);
        lines.push(`    const result = await invoke('${invokeCmd}');`);
        lines.push(`    console.log('[Tauri] ${id} clicked, result:', result);`);
        lines.push(`  } catch (err) {`);
        lines.push(`    console.error('[Tauri] ${id} error:', err);`);
        lines.push(`  }`);
        lines.push(`});`);
    }
    else if (node.kind === 'input' || node.kind === 'textarea') {
        lines.push(`widget.addEventListener('input', async (e) => {`);
        lines.push(`  const value = e.target.value;`);
        lines.push(`  try {`);
        lines.push(`    await invoke('${id}_changed', { value });`);
        lines.push(`  } catch (err) {`);
        lines.push(`    console.error('[Tauri] ${id} input error:', err);`);
        lines.push(`  }`);
        lines.push(`});`);
    }
    else if (node.kind === 'select') {
        lines.push(`widget.addEventListener('change', async (e) => {`);
        lines.push(`  const selected = e.target.value;`);
        lines.push(`  try {`);
        lines.push(`    await invoke('${id}_selected', { selected });`);
        lines.push(`  } catch (err) {`);
        lines.push(`    console.error('[Tauri] ${id} select error:', err);`);
        lines.push(`  }`);
        lines.push(`});`);
    }
    else if (node.kind === 'toggle' || node.kind === 'checkbox') {
        lines.push(`let isChecked = ${node.checked ?? false};`);
        lines.push('');
        lines.push(`widget.addEventListener('click', async () => {`);
        lines.push(`  isChecked = !isChecked;`);
        lines.push(`  widget.setAttribute('aria-checked', isChecked);`);
        lines.push(`  widget.querySelector('.ow-toggle')?.classList.toggle('on', isChecked);`);
        lines.push(`  try {`);
        lines.push(`    await invoke('${id}_toggled', { checked: isChecked });`);
        lines.push(`  } catch (err) {`);
        lines.push(`    console.error('[Tauri] ${id} toggle error:', err);`);
        lines.push(`  }`);
        lines.push(`});`);
    }
    else {
        for (const ev of (node.events ?? [])) {
            const domEv = ev.name.replace(/^on/, '').toLowerCase();
            const cmd = `${id}_${domEv}`;
            lines.push(`widget.addEventListener('${domEv}', async (e) => {`);
            lines.push(`  try {`);
            lines.push(`    await invoke('${cmd}', { data: e.detail ?? null });`);
            lines.push(`  } catch (err) {`);
            lines.push(`    console.error('[Tauri] ${cmd} error:', err);`);
            lines.push(`  }`);
            lines.push(`});`);
        }
    }
    return lines.join('\n');
}
function renderRustCommand(node) {
    const id = node.id ?? node.name?.toLowerCase() ?? 'widget';
    const fnName = id.replace(/-/g, '_');
    const lines = [];
    if (node.kind === 'button') {
        lines.push(`#[tauri::command]`);
        lines.push(`fn ${fnName}_clicked() -> Result<String, String> {`);
        lines.push(`    // Handle ${id} click`);
        lines.push(`    Ok("success".to_string())`);
        lines.push(`}`);
    }
    else if (node.kind === 'input' || node.kind === 'textarea') {
        lines.push(`#[tauri::command]`);
        lines.push(`fn ${fnName}_changed(value: String) -> Result<(), String> {`);
        lines.push(`    println!("[${id}] value changed: {}", value);`);
        lines.push(`    Ok(())`);
        lines.push(`}`);
    }
    else if (node.kind === 'toggle' || node.kind === 'checkbox') {
        lines.push(`#[tauri::command]`);
        lines.push(`fn ${fnName}_toggled(checked: bool) -> Result<(), String> {`);
        lines.push(`    println!("[${id}] toggled: {}", checked);`);
        lines.push(`    Ok(())`);
        lines.push(`}`);
    }
    else {
        lines.push(`#[tauri::command]`);
        lines.push(`fn ${fnName}_event(data: Option<String>) -> Result<(), String> {`);
        lines.push(`    println!("[${id}] event: {:?}", data);`);
        lines.push(`    Ok(())`);
        lines.push(`}`);
    }
    return lines.join('\n');
}
function generateTauri(ir) {
    const node = ir.rootWidget;
    const name = ir.name.replace(/[^A-Za-z0-9]/g, '') || 'Widget';
    const htmlWidget = renderHtmlWidget(node);
    const jsCode = renderTauriJs(node);
    const rustCode = renderRustCommand(node);
    return `<!-- ${name} — Tauri Widget
     Converted from ${ir.sourceLanguage} by Omnisystem Widget Converter
     Confidence: ${ir.confidence} -->
<!DOCTYPE html>
<html lang="en" data-theme="omni-dark">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>${name}</title>
  <link rel="stylesheet" href="omni-widgets.css" />
</head>
<body>
  <!-- Widget: ${node.kind} -->
  ${htmlWidget}

  <script src="omni-widgets.js"></script>
  <script type="module">
    ${jsCode.split('\n').join('\n    ')}
  </script>
</body>
</html>

<!-- ── Rust backend (src-tauri/src/main.rs) ───────────────────────────────── -->
<!--
${rustCode}

// Register in main.rs:
// tauri::Builder::default()
//   .invoke_handler(tauri::generate_handler![${(node.id ?? name.toLowerCase()).replace(/-/g, '_')}_clicked])
//   .run(tauri::generate_context!())
//   .expect("error running tauri app");
-->
`;
}
//# sourceMappingURL=TauriGenerator.js.map