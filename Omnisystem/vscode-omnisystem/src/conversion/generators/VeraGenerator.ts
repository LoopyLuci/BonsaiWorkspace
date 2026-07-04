// Vera Generator — converts WidgetIR to Vera (OW Component) source code
import { WidgetIR, WidgetNode, WidgetKind, WidgetEvent, WidgetProp } from '../WidgetIR';

function indent(code: string, spaces: number): string {
    const pad = ' '.repeat(spaces);
    return code.split('\n').map(l => (l.trim() ? pad + l : l)).join('\n');
}

function kindToOwClass(kind: WidgetKind, variant?: string): string {
    const variantSuffix = variant ? ` ow-btn-${variant}` : ' ow-btn-primary';
    const map: Partial<Record<WidgetKind, string>> = {
        button:     `ow-btn${variantSuffix}`,
        input:      'ow-input',
        textarea:   'ow-input',
        checkbox:   'ow-checkbox',
        radio:      'ow-radio',
        toggle:     'ow-toggle-wrap',
        select:     'ow-select',
        slider:     'ow-slider',
        card:       'ow-card',
        panel:      'ow-panel',
        modal:      'ow-modal',
        badge:      'ow-badge',
        tag:        'ow-tag',
        chip:       'ow-chip',
        progress:   'ow-progress',
        spinner:    'ow-spinner',
        label:      'ow-label',
        list:       'ow-list',
        listitem:   'ow-list-item',
        table:      'ow-table',
        form:       'ow-form',
        navbar:     'ow-navbar',
        sidebar:    'ow-sidebar',
        alert:      'ow-alert',
        toast:      'ow-toast',
        divider:    'ow-divider',
        container:  'ow-container',
    };
    return map[kind] ?? 'ow-container';
}

function kindToHtmlTag(kind: WidgetKind): string {
    const map: Partial<Record<WidgetKind, string>> = {
        button:    'button',
        input:     'input',
        textarea:  'textarea',
        select:    'select',
        label:     'span',
        divider:   'hr',
        form:      'form',
        navbar:    'nav',
        list:      'ul',
        listitem:  'li',
        table:     'table',
        progress:  'progress',
        image:     'img',
        container: 'div',
        panel:     'div',
        card:      'div',
        modal:     'div',
        badge:     'span',
        chip:      'span',
        sidebar:   'aside',
        alert:     'div',
        toast:     'div',
        spinner:   'div',
    };
    return map[kind] ?? 'div';
}

function renderVeraEvent(ev: WidgetEvent): string {
    const veraDomEvent = ev.name === 'onClick' ? 'click' :
                         ev.name === 'onChange' ? 'input' :
                         ev.name === 'onSubmit' ? 'submit' :
                         ev.name === 'onFocus' ? 'focus' :
                         ev.name === 'onBlur' ? 'blur' :
                         ev.name === 'onKeyDown' ? 'keydown' :
                         ev.name === 'onMouseEnter' ? 'mouseenter' :
                         ev.name === 'onMouseLeave' ? 'mouseleave' :
                         ev.name.replace(/^on/, '').toLowerCase();
    const handlerFn = ev.handler.startsWith('self.') ? ev.handler : `self.handle_${veraDomEvent}`;
    return `on:${veraDomEvent}={${handlerFn}}`;
}

function renderVeraNode(node: WidgetNode, indent_level: number = 2): string {
    const tag = kindToHtmlTag(node.kind);
    const owClass = kindToOwClass(node.kind, node.variant);
    const events = (node.events ?? []).map(renderVeraEvent).join(' ');
    const pad = ' '.repeat(indent_level * 4);

    // Self-closing tags
    if (node.kind === 'input') {
        const placeholder = node.placeholder ? ` placeholder="${node.placeholder}"` : '';
        const value = node.value ? ` value={self.${node.name?.toLowerCase() ?? 'value'}}` : '';
        const disabled = node.disabled ? ' disabled={self.disabled}' : '';
        return `${pad}<input class="${owClass}"${placeholder}${value}${disabled} on:input={self.handle_change} />`;
    }

    if (node.kind === 'textarea') {
        const placeholder = node.placeholder ? ` placeholder="${node.placeholder}"` : '';
        return `${pad}<textarea class="${owClass}"${placeholder} on:input={self.handle_change}>\n${pad}    {self.value}\n${pad}</textarea>`;
    }

    if (node.kind === 'divider') {
        return `${pad}<hr class="${owClass}" />`;
    }

    if (node.kind === 'toggle') {
        return `${pad}<div class="ow-toggle-wrap" role="switch" aria-checked={self.checked} tabindex="0"\n${pad}     on:click={self.handle_toggle} on:keydown={self.handle_key}>\n${pad}    <div class="ow-toggle {if self.checked { 'on' } else { '' }}" />\n${pad}</div>`;
    }

    if (node.kind === 'checkbox') {
        return `${pad}<label class="${owClass}">\n${pad}    <input type="checkbox" checked={self.checked} on:change={self.handle_change} />\n${pad}    <span class="ow-checkbox-label">{self.label}</span>\n${pad}</label>`;
    }

    if (node.kind === 'select') {
        const options = (node.options ?? []).map(opt =>
            `${pad}    <option value="${opt.value}"${opt.value === node.value ? ' selected' : ''}>${opt.label}</option>`
        ).join('\n') || `${pad}    <option value="">Select...</option>`;
        return `${pad}<select class="${owClass}" on:change={self.handle_change}>\n${options}\n${pad}</select>`;
    }

    if (node.kind === 'progress') {
        return `${pad}<div class="${owClass}">\n${pad}    <div class="ow-progress-fill" style="width: {self.value}%" />\n${pad}</div>`;
    }

    if (node.kind === 'spinner') {
        return `${pad}<div class="${owClass}" role="status" aria-label="Loading">\n${pad}    <div class="ow-spinner-ring" />\n${pad}</div>`;
    }

    if (node.kind === 'badge' || node.kind === 'chip') {
        return `${pad}<span class="${owClass}">{self.label}</span>`;
    }

    const openAttrs = [
        `class="${owClass}"`,
        events,
        node.disabled ? 'disabled={self.disabled}' : '',
        node.kind === 'button' ? `aria-label="${node.label ?? node.name ?? 'Button'}"` : '',
    ].filter(Boolean).join(' ');

    const children = (node.children ?? [])
        .map(c => renderVeraNode(c, indent_level + 1))
        .join('\n');

    const innerText = node.label ? `\n${pad}    {self.label}\n${pad}` :
                      children ? `\n${children}\n${pad}` :
                      '\n' + pad + '    {/* content */}\n' + pad;

    return `${pad}<${tag} ${openAttrs}>${innerText}</${tag}>`;
}

function renderProps(props: WidgetProp[], node: WidgetNode): string {
    const baseProps: WidgetProp[] = [];

    if (node.label !== undefined) {
        baseProps.push({ name: 'label', type: 'String', value: `"${node.label}"`, required: false });
    }
    if (node.placeholder !== undefined) {
        baseProps.push({ name: 'placeholder', type: 'String', value: `"${node.placeholder}"`, required: false });
    }
    if (node.value !== undefined) {
        baseProps.push({ name: 'value', type: 'String', value: `"${node.value}"`, required: false });
    }
    if (node.disabled) {
        baseProps.push({ name: 'disabled', type: 'Bool', value: 'false', required: false });
    }
    if (node.variant) {
        baseProps.push({ name: 'variant', type: 'String', value: `"${node.variant}"`, required: false });
    }
    if (node.size) {
        baseProps.push({ name: 'size', type: 'String', value: `"${node.size}"`, required: false });
    }

    // Add event handler props
    for (const ev of (node.events ?? [])) {
        const propName = ev.name;
        baseProps.push({ name: propName, type: 'Fn()', value: '|| {}', required: false });
    }

    const allProps = [...baseProps, ...props.filter(p => !baseProps.find(b => b.name === p.name))];

    if (allProps.length === 0) { return '    props {\n        // No props required\n    }'; }

    const lines = allProps.map(p => {
        const defaultVal = p.value ? ` = ${p.value}` : '';
        const reqMark = p.required ? '' : '?';
        return `        ${p.name}${reqMark}: ${p.type}${defaultVal}`;
    });

    return `    props {\n${lines.join('\n')}\n    }`;
}

function renderState(node: WidgetNode): string {
    const stateLines: string[] = [];

    if (node.label !== undefined)      { stateLines.push(`        label: String = "${node.label}"`); }
    if (node.placeholder !== undefined){ stateLines.push(`        placeholder: String = "${node.placeholder}"`); }
    if (['input', 'textarea', 'select'].includes(node.kind)) {
        stateLines.push(`        value: String = "${node.value ?? ''}"`);
    }
    if (node.kind === 'checkbox' || node.kind === 'toggle' || node.kind === 'radio') {
        stateLines.push(`        checked: Bool = ${node.checked ? 'true' : 'false'}`);
    }
    if (node.disabled !== undefined)   { stateLines.push(`        disabled: Bool = ${node.disabled}`); }

    if (stateLines.length === 0) { return '    state {\n        // No internal state\n    }'; }
    return `    state {\n${stateLines.join('\n')}\n    }`;
}

function renderHandlers(node: WidgetNode): string {
    const handlers: string[] = [];

    for (const ev of (node.events ?? [])) {
        const fnName = ev.name === 'onClick' ? 'handle_click' :
                       ev.name === 'onChange' ? 'handle_change' :
                       ev.name === 'onSubmit' ? 'handle_submit' :
                       `handle_${ev.name.replace(/^on/, '').toLowerCase()}`;
        handlers.push(
`    fn ${fnName}(${ev.params?.join(', ') ?? ''}) {
        self.${ev.name}()
    }`
        );
    }

    if (node.kind === 'toggle' || node.kind === 'checkbox') {
        handlers.push(
`    fn handle_toggle() {
        self.checked = !self.checked
    }

    fn handle_key(event: KeyEvent) {
        if event.key == " " || event.key == "Enter" {
            self.handle_toggle()
        }
    }`
        );
    }

    return handlers.join('\n\n') || '    // No event handlers';
}

export function generateVera(ir: WidgetIR): string {
    const node = ir.rootWidget;
    const name = ir.name.replace(/[^A-Za-z0-9]/g, '') || 'Widget';
    const capitalName = name.charAt(0).toUpperCase() + name.slice(1);

    const propsBlock   = renderProps(node.props ?? [], node);
    const stateBlock   = renderState(node);
    const handlersSection = renderHandlers(node);
    const renderBlock  = renderVeraNode(node, 1);

    return `// ${capitalName} — Vera OW Component
// Converted from ${ir.sourceLanguage} by Omnisystem Widget Converter
// Confidence: ${ir.confidence}

component ${capitalName} {

${propsBlock}

${stateBlock}

${handlersSection}

    render {
${renderBlock}
    }
}
`;
}
