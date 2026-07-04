"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.generateTs = generateTs;
function kindToOwClass(kind) {
    const map = {
        button: 'ow-btn', input: 'ow-input', textarea: 'ow-input', select: 'ow-select',
        checkbox: 'ow-checkbox', toggle: 'ow-toggle-wrap', slider: 'ow-slider',
        card: 'ow-card', panel: 'ow-panel', modal: 'ow-modal', badge: 'ow-badge',
        progress: 'ow-progress', spinner: 'ow-spinner', list: 'ow-list',
        navbar: 'ow-navbar', sidebar: 'ow-sidebar', alert: 'ow-alert', toast: 'ow-toast',
        form: 'ow-form', table: 'ow-table', divider: 'ow-divider', container: 'ow-container',
        tabgroup: 'ow-tabs', tab: 'ow-tab', chip: 'ow-chip', label: 'ow-label',
    };
    return map[kind] ?? `ow-${kind}`;
}
function kindToTag(kind) {
    const map = {
        button: 'button', input: 'input', textarea: 'textarea', select: 'select',
        label: 'span', form: 'form', navbar: 'nav', list: 'ul', listitem: 'li',
        table: 'table', divider: 'hr', sidebar: 'aside', image: 'img', badge: 'span', chip: 'span',
    };
    return map[kind] ?? 'div';
}
function renderInterface(node, name) {
    const lines = [];
    if (node.label !== undefined) {
        lines.push(`  label?: string;`);
    }
    if (node.placeholder !== undefined) {
        lines.push(`  placeholder?: string;`);
    }
    if (['input', 'textarea', 'select'].includes(node.kind)) {
        lines.push(`  value?: string;`);
        lines.push(`  defaultValue?: string;`);
        lines.push(`  onChange?: (value: string, event: Event) => void;`);
    }
    if (node.kind === 'button') {
        lines.push(`  variant?: 'primary' | 'secondary' | 'danger' | 'warning' | 'success' | 'ghost' | 'outline';`);
        lines.push(`  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl';`);
        lines.push(`  onClick?: (event: MouseEvent) => void;`);
        lines.push(`  type?: 'button' | 'submit' | 'reset';`);
    }
    if (node.kind === 'toggle' || node.kind === 'checkbox') {
        lines.push(`  checked?: boolean;`);
        lines.push(`  onChange?: (checked: boolean) => void;`);
    }
    if (node.kind === 'slider') {
        lines.push(`  min?: number;`);
        lines.push(`  max?: number;`);
        lines.push(`  step?: number;`);
        lines.push(`  value?: number;`);
        lines.push(`  onChange?: (value: number) => void;`);
    }
    if (node.kind === 'select') {
        lines.push(`  options?: Array<{ value: string; label: string; disabled?: boolean }>;`);
    }
    if (node.kind === 'progress') {
        lines.push(`  value?: number;`);
        lines.push(`  max?: number;`);
        lines.push(`  label?: string;`);
    }
    if (node.disabled !== undefined) {
        lines.push(`  disabled?: boolean;`);
    }
    lines.push(`  id?: string;`);
    lines.push(`  className?: string;`);
    lines.push(`  ariaLabel?: string;`);
    // Extra props from IR
    for (const prop of (node.props ?? [])) {
        if (!lines.some(l => l.includes(prop.name + '?'))) {
            const tsType = prop.type === 'String' ? 'string' :
                prop.type === 'Bool' ? 'boolean' :
                    prop.type === 'i64' || prop.type === 'f64' || prop.type === 'u32' ? 'number' :
                        prop.type === 'Fn()' ? '() => void' :
                            prop.type;
            lines.push(`  ${prop.name}?: ${tsType};`);
        }
    }
    return `export interface ${name}Props {\n${lines.join('\n')}\n}`;
}
function renderCreateFunction(node, name) {
    const capitalName = name.charAt(0).toUpperCase() + name.slice(1);
    const tag = kindToTag(node.kind);
    const owClass = kindToOwClass(node.kind);
    const lines = [];
    lines.push(`export function create${capitalName}(props: ${capitalName}Props = {}): HTMLElement {`);
    // Destructure props
    const destructured = [];
    if (node.label !== undefined) {
        destructured.push(`label = '${node.label ?? 'Label'}'`);
    }
    if (node.placeholder !== undefined) {
        destructured.push(`placeholder = '${node.placeholder}'`);
    }
    if (['input', 'textarea', 'select'].includes(node.kind)) {
        destructured.push(`value = ''`, `defaultValue`, `onChange`);
    }
    if (node.kind === 'button') {
        destructured.push(`variant = '${node.variant ?? 'primary'}'`, `size = '${node.size ?? 'md'}'`, `onClick`, `type: btnType = 'button'`);
    }
    if (node.kind === 'toggle' || node.kind === 'checkbox') {
        destructured.push(`checked = false`, `onChange`);
    }
    if (node.kind === 'slider') {
        destructured.push(`min = 0`, `max = 100`, `step = 1`, `value: sliderValue = 0`, `onChange`);
    }
    if (node.kind === 'select') {
        destructured.push(`options = []`);
    }
    if (node.kind === 'progress') {
        destructured.push(`value: progressValue = 0`, `max = 100`);
    }
    if (node.disabled !== undefined) {
        destructured.push(`disabled = false`);
    }
    destructured.push(`id`, `className = ''`, `ariaLabel = label`);
    if (destructured.length > 0) {
        lines.push(`  const { ${destructured.join(', ')} } = props;`);
        lines.push('');
    }
    if (node.kind === 'button') {
        lines.push(`  const el = document.createElement('button') as HTMLButtonElement;`);
        lines.push(`  el.type = btnType ?? 'button';`);
        lines.push(`  el.className = \`${owClass} ow-btn-\${variant} ow-btn-\${size}\${className ? ' ' + className : ''}\`;`);
        lines.push(`  el.textContent = label ?? 'Button';`);
        lines.push(`  el.disabled = disabled ?? false;`);
        lines.push(`  if (id) el.id = id;`);
        lines.push(`  el.setAttribute('aria-label', ariaLabel ?? label ?? 'Button');`);
        lines.push(`  if (onClick) el.addEventListener('click', onClick as EventListener);`);
    }
    else if (node.kind === 'input') {
        lines.push(`  const el = document.createElement('input') as HTMLInputElement;`);
        lines.push(`  el.className = \`${owClass}\${className ? ' ' + className : ''}\`;`);
        lines.push(`  if (placeholder) el.placeholder = placeholder;`);
        lines.push(`  el.value = value ?? defaultValue ?? '';`);
        if (node.disabled !== undefined) {
            lines.push(`  el.disabled = disabled ?? false;`);
        }
        lines.push(`  if (id) el.id = id;`);
        lines.push(`  if (ariaLabel) el.setAttribute('aria-label', ariaLabel);`);
        lines.push(`  if (onChange) el.addEventListener('input', (e) => onChange((e.target as HTMLInputElement).value, e));`);
    }
    else if (node.kind === 'textarea') {
        lines.push(`  const el = document.createElement('textarea') as HTMLTextAreaElement;`);
        lines.push(`  el.className = \`${owClass}\${className ? ' ' + className : ''}\`;`);
        lines.push(`  if (placeholder) el.placeholder = placeholder;`);
        lines.push(`  el.value = value ?? '';`);
        lines.push(`  if (onChange) el.addEventListener('input', (e) => onChange((e.target as HTMLTextAreaElement).value, e));`);
    }
    else if (node.kind === 'toggle') {
        lines.push(`  let isChecked = checked ?? false;`);
        lines.push(`  const wrap = document.createElement('div');`);
        lines.push(`  wrap.className = \`ow-toggle-wrap\${className ? ' ' + className : ''}\`;`);
        lines.push(`  wrap.setAttribute('role', 'switch');`);
        lines.push(`  wrap.setAttribute('aria-checked', String(isChecked));`);
        lines.push(`  wrap.setAttribute('tabindex', '0');`);
        lines.push(`  if (id) wrap.id = id;`);
        lines.push(`  if (ariaLabel) wrap.setAttribute('aria-label', ariaLabel);`);
        lines.push(`  const knob = document.createElement('div');`);
        lines.push(`  knob.className = \`ow-toggle\${isChecked ? ' on' : ''}\`;`);
        lines.push(`  wrap.appendChild(knob);`);
        lines.push(`  const doToggle = (): void => {`);
        lines.push(`    isChecked = !isChecked;`);
        lines.push(`    wrap.setAttribute('aria-checked', String(isChecked));`);
        lines.push(`    knob.classList.toggle('on', isChecked);`);
        lines.push(`    onChange?.(isChecked);`);
        lines.push(`  };`);
        lines.push(`  wrap.addEventListener('click', doToggle);`);
        lines.push(`  wrap.addEventListener('keydown', (e: KeyboardEvent) => {`);
        lines.push(`    if (e.key === ' ' || e.key === 'Enter') { e.preventDefault(); doToggle(); }`);
        lines.push(`  });`);
        lines.push(`  return wrap;`);
    }
    else if (node.kind === 'select') {
        lines.push(`  const el = document.createElement('select') as HTMLSelectElement;`);
        lines.push(`  el.className = \`${owClass}\${className ? ' ' + className : ''}\`;`);
        lines.push(`  (options ?? []).forEach(({ value: v, label: l, disabled: d }) => {`);
        lines.push(`    const opt = new Option(l, v);`);
        lines.push(`    if (d) opt.disabled = true;`);
        lines.push(`    el.add(opt);`);
        lines.push(`  });`);
        lines.push(`  if (onChange) el.addEventListener('change', (e) => onChange((e.target as HTMLSelectElement).value, e));`);
    }
    else if (node.kind === 'slider') {
        lines.push(`  const el = document.createElement('input') as HTMLInputElement;`);
        lines.push(`  el.type = 'range';`);
        lines.push(`  el.className = \`${owClass}\${className ? ' ' + className : ''}\`;`);
        lines.push(`  el.min = String(min ?? 0);`);
        lines.push(`  el.max = String(max ?? 100);`);
        lines.push(`  el.step = String(step ?? 1);`);
        lines.push(`  el.value = String(sliderValue ?? 0);`);
        lines.push(`  if (onChange) el.addEventListener('input', (e) => onChange(Number((e.target as HTMLInputElement).value), e));`);
    }
    else if (node.kind === 'progress') {
        lines.push(`  const wrap = document.createElement('div');`);
        lines.push(`  wrap.className = \`${owClass}\${className ? ' ' + className : ''}\`;`);
        lines.push(`  wrap.setAttribute('role', 'progressbar');`);
        lines.push(`  wrap.setAttribute('aria-valuenow', String(progressValue ?? 0));`);
        lines.push(`  wrap.setAttribute('aria-valuemax', String(max ?? 100));`);
        lines.push(`  const fill = document.createElement('div');`);
        lines.push(`  fill.className = 'ow-progress-fill';`);
        lines.push(`  fill.style.width = \`\${((progressValue ?? 0) / (max ?? 100)) * 100}%\`;`);
        lines.push(`  wrap.appendChild(fill);`);
        lines.push(`  return wrap;`);
    }
    else if (node.kind === 'card') {
        lines.push(`  const card = document.createElement('div');`);
        lines.push(`  card.className = \`ow-card\${className ? ' ' + className : ''}\`;`);
        lines.push(`  if (id) card.id = id;`);
        lines.push(`  if (label) {`);
        lines.push(`    const title = document.createElement('div');`);
        lines.push(`    title.className = 'ow-card-title';`);
        lines.push(`    title.textContent = label;`);
        lines.push(`    card.appendChild(title);`);
        lines.push(`  }`);
        lines.push(`  const body = document.createElement('div');`);
        lines.push(`  body.className = 'ow-card-body';`);
        lines.push(`  card.appendChild(body);`);
        lines.push(`  return card;`);
    }
    else {
        lines.push(`  const el = document.createElement('${tag}');`);
        lines.push(`  el.className = \`${owClass}\${className ? ' ' + className : ''}\`;`);
        if (node.label !== undefined) {
            lines.push(`  if (label) el.textContent = label;`);
        }
        if (node.disabled !== undefined) {
            lines.push(`  if (disabled && 'disabled' in el) (el as HTMLButtonElement).disabled = true;`);
        }
        lines.push(`  if (id) el.id = id;`);
    }
    if (node.kind !== 'toggle' && node.kind !== 'progress' && node.kind !== 'card') {
        lines.push(`  return el;`);
    }
    lines.push(`}`);
    return lines.join('\n');
}
function generateTs(ir) {
    const node = ir.rootWidget;
    const name = ir.name.replace(/[^A-Za-z0-9]/g, '') || 'Widget';
    const capitalName = name.charAt(0).toUpperCase() + name.slice(1);
    const iface = renderInterface(node, capitalName);
    const factory = renderCreateFunction(node, name);
    return `// ${capitalName} — OW Widget (TypeScript)
// Converted from ${ir.sourceLanguage} by Omnisystem Widget Converter
// Confidence: ${ir.confidence}
// Requires: omni-widgets.css + omni-widgets.js (or OW bundle)

${iface}

${factory}

// ── Usage ──────────────────────────────────────────────────────────────────────
// const widget = create${capitalName}({${node.label !== undefined ? `\n//   label: '${node.label ?? name}',` : ''}${node.events?.find(e => e.name === 'onClick') ? `\n//   onClick: (e) => console.log('clicked', e),` : ''}${node.events?.find(e => e.name === 'onChange') ? `\n//   onChange: (v) => console.log('changed:', v),` : ''}
// });
// document.body.appendChild(widget);
`;
}
//# sourceMappingURL=TsGenerator.js.map