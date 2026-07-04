"use strict";
// JavaScript / TypeScript / JSX / TSX Parser
// Detects widget patterns in JS, TS, React, Vue, Svelte source code
Object.defineProperty(exports, "__esModule", { value: true });
exports.parseJs = parseJs;
const WidgetIR_1 = require("../WidgetIR");
const JSX_TAG_TO_KIND = {
    'button': 'button', 'Button': 'button', 'Btn': 'button',
    'input': 'input', 'Input': 'input', 'TextField': 'input', 'TextInput': 'input',
    'textarea': 'textarea', 'TextArea': 'textarea', 'Textarea': 'textarea',
    'select': 'select', 'Select': 'select', 'Dropdown': 'select', 'ComboBox': 'select',
    'checkbox': 'checkbox', 'Checkbox': 'checkbox', 'CheckBox': 'checkbox',
    'radio': 'radio', 'Radio': 'radio', 'RadioButton': 'radio',
    'Switch': 'toggle', 'Toggle': 'toggle', 'ToggleButton': 'toggle',
    'Slider': 'slider', 'Range': 'slider', 'RangeInput': 'slider',
    'Card': 'card', 'CardView': 'card', 'Panel': 'panel',
    'Modal': 'modal', 'Dialog': 'modal', 'Drawer': 'drawer', 'Sheet': 'drawer',
    'Tab': 'tab', 'TabPanel': 'tab', 'Tabs': 'tabgroup', 'TabGroup': 'tabgroup',
    'List': 'list', 'FlatList': 'list', 'VirtualList': 'list', 'ListItem': 'listitem',
    'Table': 'table', 'DataTable': 'table', 'Grid': 'grid', 'DataGrid': 'grid',
    'Form': 'form', 'Label': 'label', 'Badge': 'badge', 'Tag': 'tag', 'Chip': 'chip',
    'Progress': 'progress', 'ProgressBar': 'progress', 'Spinner': 'spinner', 'Loader': 'spinner',
    'Icon': 'icon', 'Image': 'image', 'Avatar': 'avatar',
    'Tooltip': 'tooltip', 'Popover': 'popover', 'Toast': 'toast', 'Alert': 'alert',
    'NavBar': 'navbar', 'Navbar': 'navbar', 'Nav': 'navbar', 'Navigation': 'navbar',
    'Sidebar': 'sidebar',
    'div': 'container', 'section': 'panel', 'article': 'card', 'aside': 'sidebar',
    'nav': 'navbar', 'header': 'navbar', 'footer': 'panel',
    'span': 'label', 'p': 'label', 'h1': 'label', 'h2': 'label', 'h3': 'label',
    'a': 'button', 'Link': 'button', 'NavLink': 'button',
    'img': 'image', 'Img': 'image', 'Picture': 'image',
    'Breadcrumb': 'breadcrumb', 'Pagination': 'pagination', 'Stepper': 'stepper',
    'Rating': 'rating', 'Star': 'rating', 'ColorPicker': 'colorpicker',
    'DatePicker': 'datepicker', 'Calendar': 'calendar',
    'hr': 'divider', 'Divider': 'divider', 'Separator': 'divider',
};
const EVENT_MAP = {
    'onClick': 'onClick', 'on:click': 'onClick', '@click': 'onClick',
    'onChange': 'onChange', 'on:change': 'onChange', '@change': 'onChange', 'onInput': 'onChange',
    'onSubmit': 'onSubmit', 'on:submit': 'onSubmit', '@submit': 'onSubmit',
    'onFocus': 'onFocus', 'onBlur': 'onBlur', 'onKeyDown': 'onKeyDown',
    'onKeyUp': 'onKeyUp', 'onKeyPress': 'onKeyPress',
    'onMouseEnter': 'onMouseEnter', 'onMouseLeave': 'onMouseLeave',
    'onScroll': 'onScroll', 'onResize': 'onResize',
};
function extractStringAttr(src, attrName) {
    // Matches: attrName="value" or attrName='value' or attrName={`value`} or attrName={"value"}
    const patterns = [
        new RegExp(`${attrName}=["']([^"']+)["']`, 'i'),
        new RegExp(`${attrName}=\\{["'\`]([^"'\`]+)["'\`]\\}`, 'i'),
        new RegExp(`${attrName}=\\{([^{}]+)\\}`, 'i'),
    ];
    for (const p of patterns) {
        const m = src.match(p);
        if (m) {
            return m[1].trim();
        }
    }
    return undefined;
}
function extractBoolAttr(src, attrName) {
    return new RegExp(`\\b${attrName}(=\\{true\\}|=["']true["']|(?=[\\s/>]))`, 'i').test(src);
}
function extractEvents(src) {
    const events = [];
    const eventRx = /\b(on[A-Z]\w+|on:[a-z]+|@[a-z]+)\s*=\s*\{([^}]+)\}/g;
    let m;
    while ((m = eventRx.exec(src)) !== null) {
        const rawName = m[1];
        const handler = m[2].trim().slice(0, 200);
        const normalized = EVENT_MAP[rawName] ?? rawName;
        events.push({ name: normalized, handler, params: [] });
    }
    return events;
}
function extractStyleFromClassName(className) {
    const result = {};
    if (/primary/i.test(className)) {
        result.variant = 'primary';
    }
    else if (/secondary/i.test(className)) {
        result.variant = 'secondary';
    }
    else if (/danger|destructive|error/i.test(className)) {
        result.variant = 'danger';
    }
    else if (/warning/i.test(className)) {
        result.variant = 'warning';
    }
    else if (/success/i.test(className)) {
        result.variant = 'success';
    }
    else if (/ghost|outline/i.test(className)) {
        result.variant = 'ghost';
    }
    if (/\bsm\b|small/i.test(className)) {
        result.size = 'sm';
    }
    else if (/\blg\b|large/i.test(className)) {
        result.size = 'lg';
    }
    else if (/\bxl\b/i.test(className)) {
        result.size = 'xl';
    }
    else if (/\bxs\b/i.test(className)) {
        result.size = 'xs';
    }
    return result;
}
function detectFromJSX(src) {
    // Find first JSX element
    const jsxRx = /<([A-Z][A-Za-z0-9.]*|[a-z]+)(\s[^>]*)?\s*(?:\/?>|>)/;
    const m = src.match(jsxRx);
    if (!m) {
        return null;
    }
    const tagName = m[1];
    const kind = JSX_TAG_TO_KIND[tagName] ?? 'unknown';
    const attrBlock = m[2] ?? '';
    const fullMatch = m[0];
    const label = extractStringAttr(attrBlock, 'children') ??
        extractStringAttr(fullMatch, 'label') ??
        extractStringAttr(attrBlock, 'title') ??
        extractStringAttr(attrBlock, 'text') ??
        (() => {
            // Extract text content between tags
            const textRx = new RegExp(`<${tagName}[^>]*>([^<]{1,100})</${tagName}>`, 's');
            const t = src.match(textRx);
            return t ? t[1].trim() : undefined;
        })();
    const placeholder = extractStringAttr(attrBlock, 'placeholder');
    const value = extractStringAttr(attrBlock, 'value') ?? extractStringAttr(attrBlock, 'defaultValue');
    const disabled = extractBoolAttr(attrBlock, 'disabled');
    const events = extractEvents(attrBlock + ' ' + src.slice(0, 600));
    const className = extractStringAttr(attrBlock, 'className') ?? extractStringAttr(attrBlock, 'class') ?? '';
    const { variant, size } = extractStyleFromClassName(className);
    // Name from component name or containing function/const
    const nameMx = src.match(/(?:const|let|function|export\s+(?:default\s+)?(?:function|const))\s+([A-Z][A-Za-z0-9]*)/);
    const name = nameMx ? nameMx[1] : tagName;
    // Props from interface/type definition
    const props = [];
    const ifaceRx = /(?:interface|type)\s+\w*Props\w*\s*\{([^}]+)\}/s;
    const ifaceM = src.match(ifaceRx);
    if (ifaceM) {
        const propLines = ifaceM[1].split('\n').filter(l => l.includes(':'));
        for (const line of propLines) {
            const pm = line.match(/(\w+)\??\s*:\s*([^;,]+)/);
            if (pm) {
                props.push({
                    name: pm[1].trim(),
                    type: pm[2].trim(),
                    required: !line.includes('?'),
                });
            }
        }
    }
    // Options from select/enum
    const options = [];
    if (kind === 'select') {
        const optRx = /<option[^>]*value=["']([^"']+)["'][^>]*>([^<]+)<\/option>/g;
        let om;
        while ((om = optRx.exec(src)) !== null) {
            options.push({ value: om[1], label: om[2].trim() });
        }
    }
    const confidence = kind !== 'unknown' ? (events.length > 0 ? 'high' : 'medium') : 'low';
    const notes = [];
    if (tagName[0] === tagName[0].toUpperCase()) {
        notes.push(`Detected React/component: <${tagName}>`);
    }
    if (props.length > 0) {
        notes.push(`Extracted ${props.length} props from type definition`);
    }
    return { kind, name, label, placeholder, value, disabled, variant, size, events, props, className, options, confidence, notes };
}
function detectFromDocumentCreateElement(src) {
    const rx = /document\.createElement\s*\(\s*['"]([a-z]+)['"]\s*\)/;
    const m = src.match(rx);
    if (!m) {
        return null;
    }
    const tagName = m[1];
    const kind = JSX_TAG_TO_KIND[tagName] ?? 'unknown';
    // Look for textContent / innerText / innerHTML assignment
    const labelRx = /\.(?:textContent|innerText|innerHTML)\s*=\s*['"`]([^'"`]+)['"`]/;
    const label = src.match(labelRx)?.[1];
    const events = extractEvents(src);
    const addListenerRx = /addEventListener\s*\(\s*['"]([^'"]+)['"],\s*([^)]+)\)/g;
    let am;
    while ((am = addListenerRx.exec(src)) !== null) {
        const evName = 'on' + am[1].charAt(0).toUpperCase() + am[1].slice(1);
        if (!events.find(e => e.name === evName)) {
            events.push({ name: evName, handler: am[2].trim().slice(0, 100), params: ['event'] });
        }
    }
    const nameMx = src.match(/(?:const|let|var)\s+(\w+)\s*=\s*document\.createElement/);
    const name = nameMx ? nameMx[1].replace(/^\w/, c => c.toUpperCase()) : tagName;
    return {
        kind, name, label, events, props: [], className: '',
        confidence: kind !== 'unknown' ? 'medium' : 'low',
        notes: ['DOM manipulation pattern detected'],
    };
}
function detectFromClassComponent(src) {
    const rx = /class\s+(\w+)\s+extends\s+(?:React\.)?(?:Component|PureComponent)/;
    const m = src.match(rx);
    if (!m) {
        return null;
    }
    const name = m[1];
    // Look for render() → JSX
    const renderM = src.match(/render\s*\(\s*\)\s*\{([\s\S]+)\}/);
    if (renderM) {
        const inner = detectFromJSX(renderM[1]);
        if (inner) {
            return { ...inner, name, notes: [...inner.notes, 'React class component'] };
        }
    }
    return {
        kind: 'container', name, events: [], props: [],
        confidence: 'low', notes: ['React class component detected'],
    };
}
function detectFromFunctionalComponent(src) {
    // Arrow function or function component returning JSX
    const rx = /(?:const|function)\s+([A-Z][A-Za-z0-9]*)\s*(?:=\s*(?:\([^)]*\)|[^=]+)\s*=>\s*|(?:\([^)]*\))\s*\{)/;
    const m = src.match(rx);
    if (!m) {
        return null;
    }
    const name = m[1];
    const inner = detectFromJSX(src);
    if (inner) {
        return { ...inner, name, notes: [...inner.notes, 'Functional component'] };
    }
    return null;
}
function detectFromSvelte(src) {
    // Svelte: on:click, bind:value, etc.
    if (!src.includes('on:') && !src.includes('<script>')) {
        return null;
    }
    const inner = detectFromJSX(src);
    if (inner) {
        return { ...inner, notes: [...inner.notes, 'Svelte component pattern'] };
    }
    return null;
}
function detectFromVue(src) {
    if (!src.includes('@click') && !src.includes('v-on') && !src.includes('<template>')) {
        return null;
    }
    const inner = detectFromJSX(src);
    if (inner) {
        return { ...inner, notes: [...inner.notes, 'Vue component pattern'] };
    }
    return null;
}
function detectedToNode(d) {
    return {
        id: (0, WidgetIR_1.makeId)(d.name || d.kind),
        kind: d.kind,
        name: d.name,
        label: d.label,
        placeholder: d.placeholder,
        value: d.value,
        disabled: d.disabled,
        variant: d.variant,
        size: d.size,
        events: d.events,
        props: d.props,
        className: d.className,
        options: d.options,
        children: d.children?.map(detectedToNode),
    };
}
function parseJs(source, lang = 'javascript') {
    const src = source.trim();
    const notes = [];
    let detected = detectFromFunctionalComponent(src) ??
        detectFromClassComponent(src) ??
        detectFromSvelte(src) ??
        detectFromVue(src) ??
        detectFromJSX(src) ??
        detectFromDocumentCreateElement(src);
    if (!detected) {
        // Fallback: best-guess from file content
        detected = {
            kind: 'container',
            name: 'Widget',
            events: extractEvents(src),
            props: [],
            confidence: 'low',
            notes: ['No clear widget pattern detected — treating as container'],
        };
    }
    // Check for TypeScript specifics
    if (lang === 'typescript' && src.includes('interface ')) {
        notes.push('TypeScript interfaces detected');
    }
    const imports = [];
    const importRx = /import\s+[^from]+from\s+['"]([^'"]+)['"]/g;
    let im;
    while ((im = importRx.exec(src)) !== null) {
        imports.push(im[1]);
    }
    return {
        name: detected.name || 'Widget',
        rootWidget: detectedToNode(detected),
        imports,
        sourceLanguage: lang,
        confidence: detected.confidence,
        notes: [...detected.notes, ...notes],
    };
}
//# sourceMappingURL=JsParser.js.map