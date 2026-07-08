"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.generateJs = generateJs;
function kindToOwClass(kind, variant) {
    const varSuffix = variant ? ` ow-btn-${variant}` : kind === 'button' ? ' ow-btn-primary' : '';
    const base = kind === 'button' ? 'ow-btn' :
        kind === 'input' ? 'ow-input' :
            kind === 'textarea' ? 'ow-input' :
                kind === 'select' ? 'ow-select' :
                    kind === 'checkbox' ? 'ow-checkbox' :
                        kind === 'toggle' ? 'ow-toggle-wrap' :
                            kind === 'slider' ? 'ow-slider' :
                                kind === 'card' ? 'ow-card' :
                                    kind === 'panel' ? 'ow-panel' :
                                        kind === 'modal' ? 'ow-modal' :
                                            kind === 'badge' ? 'ow-badge' :
                                                kind === 'progress' ? 'ow-progress' :
                                                    kind === 'spinner' ? 'ow-spinner' :
                                                        kind === 'list' ? 'ow-list' :
                                                            kind === 'navbar' ? 'ow-navbar' :
                                                                kind === 'alert' ? 'ow-alert' :
                                                                    kind === 'toast' ? 'ow-toast' :
                                                                        kind === 'divider' ? 'ow-divider' :
                                                                            kind === 'form' ? 'ow-form' :
                                                                                kind === 'tabgroup' ? 'ow-tabs' :
                                                                                    kind === 'table' ? 'ow-table' :
                                                                                        `ow-${kind}`;
    return base + varSuffix;
}
function kindToTag(kind) {
    const map = {
        button: 'button', input: 'input', textarea: 'textarea', select: 'select',
        label: 'span', form: 'form', navbar: 'nav', list: 'ul', listitem: 'li',
        table: 'table', divider: 'hr', sidebar: 'aside', image: 'img', progress: 'div',
        badge: 'span', chip: 'span',
    };
    return map[kind] ?? 'div';
}
function renderJsWidget(node) {
    const tag = kindToTag(node.kind);
    const owClass = kindToOwClass(node.kind, node.variant);
    const fnName = `create${(node.name ?? node.kind).replace(/[^A-Za-z0-9]/g, '').replace(/^\w/, c => c.toUpperCase())}`;
    const paramDocs = [];
    const paramDefaults = [];
    if (node.label !== undefined) {
        paramDocs.push(` * @param {string} [label="${node.label ?? 'Label'}"] - Display text`);
        paramDefaults.push(`label = '${node.label ?? 'Label'}'`);
    }
    if (node.kind === 'input' || node.kind === 'textarea') {
        paramDocs.push(` * @param {string} [placeholder="${node.placeholder ?? ''}"] - Placeholder text`);
        paramDefaults.push(`placeholder = '${node.placeholder ?? ''}'`);
        paramDocs.push(` * @param {string} [value=""] - Initial value`);
        paramDefaults.push(`value = ''`);
    }
    if (node.kind === 'button') {
        paramDocs.push(` * @param {'primary'|'secondary'|'danger'|'ghost'} [variant='${node.variant ?? 'primary'}'] - Button style`);
        paramDefaults.push(`variant = '${node.variant ?? 'primary'}'`);
        paramDocs.push(` * @param {'sm'|'md'|'lg'} [size='${node.size ?? 'md'}'] - Button size`);
        paramDefaults.push(`size = '${node.size ?? 'md'}'`);
    }
    if (node.disabled !== undefined) {
        paramDocs.push(` * @param {boolean} [disabled=false] - Disabled state`);
        paramDefaults.push(`disabled = false`);
    }
    // Event handler params
    for (const ev of (node.events ?? [])) {
        const evParam = ev.name.charAt(0).toLowerCase() + ev.name.slice(1);
        paramDocs.push(` * @param {Function} [${evParam}=null] - ${ev.name} handler`);
        paramDefaults.push(`${evParam} = null`);
    }
    const paramsStr = paramDefaults.length > 0 ? `{\n    ${paramDefaults.join(',\n    ')}\n  } = {}` : '';
    const docLines = paramDocs.length > 0 ? `\n${paramDocs.join('\n')}` : '';
    // Generate element creation
    const bodyLines = [];
    if (node.kind === 'input' || node.kind === 'textarea') {
        bodyLines.push(`  const el = document.createElement('${tag}');`);
        bodyLines.push(`  el.className = '${owClass}';`);
        if (node.placeholder !== undefined) {
            bodyLines.push(`  el.placeholder = placeholder;`);
        }
        bodyLines.push(`  el.value = value;`);
        if (node.disabled !== undefined) {
            bodyLines.push(`  if (disabled) el.setAttribute('disabled', '');`);
        }
        for (const ev of (node.events ?? [])) {
            const evParam = ev.name.charAt(0).toLowerCase() + ev.name.slice(1);
            const domEvent = ev.name === 'onChange' ? 'input' : ev.name.replace(/^on/, '').toLowerCase();
            bodyLines.push(`  if (${evParam}) el.addEventListener('${domEvent}', (e) => ${evParam}(e.target.value, e));`);
        }
        bodyLines.push(`  return el;`);
    }
    else if (node.kind === 'button') {
        bodyLines.push(`  const el = document.createElement('button');`);
        bodyLines.push(`  el.type = 'button';`);
        bodyLines.push(`  el.className = \`ow-btn ow-btn-\${variant} ow-btn-\${size}\`;`);
        if (node.label !== undefined) {
            bodyLines.push(`  el.textContent = label;`);
        }
        bodyLines.push(`  el.setAttribute('aria-label', label);`);
        if (node.disabled !== undefined) {
            bodyLines.push(`  el.disabled = disabled;`);
        }
        for (const ev of (node.events ?? [])) {
            if (ev.name === 'onClick') {
                bodyLines.push(`  if (onClick) el.addEventListener('click', onClick);`);
            }
        }
        bodyLines.push(`  return el;`);
    }
    else if (node.kind === 'toggle') {
        bodyLines.push(`  let checked = false;`);
        bodyLines.push(`  const wrap = document.createElement('div');`);
        bodyLines.push(`  wrap.className = 'ow-toggle-wrap';`);
        bodyLines.push(`  wrap.setAttribute('role', 'switch');`);
        bodyLines.push(`  wrap.setAttribute('aria-checked', 'false');`);
        bodyLines.push(`  wrap.setAttribute('tabindex', '0');`);
        bodyLines.push(`  const knob = document.createElement('div');`);
        bodyLines.push(`  knob.className = 'ow-toggle';`);
        bodyLines.push(`  wrap.appendChild(knob);`);
        bodyLines.push(`  function toggle() {`);
        bodyLines.push(`    checked = !checked;`);
        bodyLines.push(`    wrap.setAttribute('aria-checked', checked);`);
        bodyLines.push(`    knob.classList.toggle('on', checked);`);
        bodyLines.push(`    if (onChange) onChange(checked);`);
        bodyLines.push(`  }`);
        bodyLines.push(`  wrap.addEventListener('click', toggle);`);
        bodyLines.push(`  wrap.addEventListener('keydown', (e) => { if (e.key === ' ' || e.key === 'Enter') { e.preventDefault(); toggle(); } });`);
        bodyLines.push(`  return wrap;`);
    }
    else if (node.kind === 'select') {
        bodyLines.push(`  const el = document.createElement('select');`);
        bodyLines.push(`  el.className = '${owClass}';`);
        const options = node.options ?? [{ value: '', label: 'Select...' }];
        for (const opt of options) {
            bodyLines.push(`  el.add(new Option('${opt.label}', '${opt.value}'));`);
        }
        for (const ev of (node.events ?? [])) {
            if (ev.name === 'onChange') {
                bodyLines.push(`  if (onChange) el.addEventListener('change', (e) => onChange(e.target.value));`);
            }
        }
        bodyLines.push(`  return el;`);
    }
    else if (node.kind === 'card') {
        bodyLines.push(`  const card = document.createElement('div');`);
        bodyLines.push(`  card.className = 'ow-card';`);
        bodyLines.push(`  if (label) {`);
        bodyLines.push(`    const title = document.createElement('div');`);
        bodyLines.push(`    title.className = 'ow-card-title';`);
        bodyLines.push(`    title.textContent = label;`);
        bodyLines.push(`    card.appendChild(title);`);
        bodyLines.push(`  }`);
        bodyLines.push(`  const body = document.createElement('div');`);
        bodyLines.push(`  body.className = 'ow-card-body';`);
        bodyLines.push(`  card.appendChild(body);`);
        bodyLines.push(`  if (onClick) { card.style.cursor = 'pointer'; card.addEventListener('click', onClick); }`);
        bodyLines.push(`  return card;`);
    }
    else {
        bodyLines.push(`  const el = document.createElement('${tag}');`);
        bodyLines.push(`  el.className = '${owClass}';`);
        if (node.label !== undefined) {
            bodyLines.push(`  el.textContent = label;`);
        }
        for (const ev of (node.events ?? [])) {
            const evParam = ev.name.charAt(0).toLowerCase() + ev.name.slice(1);
            const domEv = ev.name.replace(/^on/, '').toLowerCase();
            bodyLines.push(`  if (${evParam}) el.addEventListener('${domEv}', ${evParam});`);
        }
        bodyLines.push(`  return el;`);
    }
    return `/**
 * Create an OW-styled ${node.kind} widget.${docLines}
 * @returns {HTMLElement}
 */
export function ${fnName}(${paramsStr}) {
${bodyLines.join('\n')}
}`;
}
function renderOWRequirement() {
    return `// Requires: omni-widgets.js and omni-widgets.css loaded in page
// <link rel="stylesheet" href="omni-widgets.css" />
// <script src="omni-widgets.js"></script>
// Or import via bundler: import 'omni-widgets.css'; import OW from 'omni-widgets.js';

`;
}
function generateJs(ir) {
    const node = ir.rootWidget;
    const name = ir.name.replace(/[^A-Za-z0-9]/g, '') || 'Widget';
    return `// ${name} — OW Widget (JavaScript)
// Converted from ${ir.sourceLanguage} by Omnisystem Widget Converter
// Confidence: ${ir.confidence}

${renderOWRequirement()}${renderJsWidget(node)}

// ── Usage example ──────────────────────────────────────────────────────────────
// const widget = create${name.charAt(0).toUpperCase() + name.slice(1)}({${node.label !== undefined ? `\n//   label: '${node.label ?? name}',` : ''}${node.events?.find(e => e.name === 'onClick') ? `\n//   onClick: () => console.log('clicked'),` : ''}${node.events?.find(e => e.name === 'onChange') ? `\n//   onChange: (v) => console.log('changed:', v),` : ''}
// });
// document.body.appendChild(widget);
`;
}
//# sourceMappingURL=JsGenerator.js.map