"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.generateTitan = generateTitan;
function propToTitanType(type) {
    const map = {
        'string': 'String', 'String': 'String', 'str': 'String',
        'number': 'f64', 'int': 'i64', 'float': 'f64', 'u32': 'u32', 'i32': 'i32',
        'boolean': 'bool', 'Bool': 'bool', 'bool': 'bool',
        'Function': 'Fn()', 'Fn()': 'Fn()', 'fn()': 'Fn()',
        'object': 'HashMap<String, String>',
        'any': 'Option<String>',
        'Vec<String>': 'Vec<String>',
        'Option<String>': 'Option<String>',
    };
    return map[type] ?? type;
}
function kindToTitanStruct(kind) {
    const map = {
        button: 'ButtonWidget',
        input: 'InputWidget',
        textarea: 'TextareaWidget',
        checkbox: 'CheckboxWidget',
        radio: 'RadioWidget',
        toggle: 'ToggleWidget',
        select: 'SelectWidget',
        slider: 'SliderWidget',
        card: 'CardWidget',
        panel: 'PanelWidget',
        modal: 'ModalWidget',
        badge: 'BadgeWidget',
        chip: 'ChipWidget',
        progress: 'ProgressWidget',
        spinner: 'SpinnerWidget',
        label: 'LabelWidget',
        list: 'ListWidget',
        table: 'TableWidget',
        form: 'FormWidget',
        navbar: 'NavbarWidget',
        sidebar: 'SidebarWidget',
        alert: 'AlertWidget',
        toast: 'ToastWidget',
        divider: 'DividerWidget',
        container: 'ContainerWidget',
        tabgroup: 'TabGroupWidget',
        tab: 'TabWidget',
    };
    return map[kind] ?? 'Widget';
}
function kindToHtmlTag(kind) {
    const map = {
        button: 'button', input: 'input', textarea: 'textarea', select: 'select',
        label: 'span', form: 'form', navbar: 'nav', list: 'ul', listitem: 'li',
        table: 'table', divider: 'hr', sidebar: 'aside', image: 'img',
    };
    return map[kind] ?? 'div';
}
function renderStructFields(node) {
    const fields = [];
    if (node.label !== undefined) {
        fields.push(`    label: String,`);
    }
    if (node.placeholder !== undefined) {
        fields.push(`    placeholder: String,`);
    }
    if (['input', 'textarea', 'select'].includes(node.kind)) {
        fields.push(`    value: String,`);
    }
    if (node.kind === 'button' || node.kind === 'toggle' || node.kind === 'checkbox') {
        fields.push(`    variant: WidgetVariant,`);
    }
    if (node.kind === 'toggle' || node.kind === 'checkbox' || node.kind === 'radio') {
        fields.push(`    checked: bool,`);
    }
    if (node.disabled !== undefined) {
        fields.push(`    disabled: bool,`);
    }
    if (node.size) {
        fields.push(`    size: WidgetSize,`);
    }
    for (const prop of (node.props ?? [])) {
        if (!fields.some(f => f.includes(prop.name + ':'))) {
            fields.push(`    ${prop.name}: ${propToTitanType(prop.type)},`);
        }
    }
    if (node.children && node.children.length > 0) {
        fields.push(`    children: Vec<Box<dyn Widget>>,`);
    }
    return fields.join('\n') || '    // No fields';
}
function renderImpl(node, structName) {
    const methods = [];
    // Constructor
    const ctorArgs = [];
    if (node.label !== undefined) {
        ctorArgs.push('label: &str');
    }
    if (node.disabled !== undefined) {
        ctorArgs.push('disabled: bool');
    }
    const ctorBody = [];
    if (node.label !== undefined) {
        ctorBody.push(`            label: label.to_string(),`);
    }
    if (node.placeholder !== undefined) {
        ctorBody.push(`            placeholder: String::new(),`);
    }
    if (['input', 'textarea', 'select'].includes(node.kind)) {
        ctorBody.push(`            value: String::new(),`);
    }
    if (node.kind === 'toggle' || node.kind === 'checkbox') {
        ctorBody.push(`            checked: false,`);
    }
    if (node.disabled !== undefined) {
        ctorBody.push(`            disabled,`);
    }
    if (node.kind === 'button') {
        ctorBody.push(`            variant: WidgetVariant::Primary,`);
    }
    methods.push(`    pub fn new(${ctorArgs.join(', ')}) -> Self {\n        Self {\n${ctorBody.join('\n')}\n        }\n    }`);
    // Event handlers
    for (const ev of (node.events ?? [])) {
        const fnName = ev.name === 'onClick' ? 'on_click' :
            ev.name === 'onChange' ? 'on_change' :
                ev.name === 'onSubmit' ? 'on_submit' :
                    ev.name.replace(/^on/, 'on_').toLowerCase();
        const paramType = ev.name === 'onChange' ? 'value: &str' :
            ev.name === 'onKeyDown' ? 'key: &str' : '';
        const genericPart = ev.handler.startsWith('self.') ? '' :
            '<F: Fn(' + (paramType ? paramType.split(':')[1].trim() : '()') + ')>';
        const dispatchExpr = ev.name === 'onChange'
            ? `self.emit("change", value)`
            : ev.name === 'onSubmit'
                ? `self.emit("submit", ())`
                : `self.emit("${ev.name.replace(/^on/, '').toLowerCase()}", ())`;
        methods.push(`    pub fn ${fnName}${genericPart}(${paramType}) {\n        ${dispatchExpr}\n    }`);
    }
    // Render method — emits OW HTML string (build without nested backticks)
    const owClass = `ow-${node.kind}`;
    const variantClass = node.kind === 'button' ? ' ow-btn-primary' : '';
    const typeAttr = node.kind === 'button' ? ' type="button"' : '';
    const tag = kindToHtmlTag(node.kind);
    const selfLabel = node.label !== undefined ? 'self.label' :
        node.kind === 'button' ? '"Button"' : '""';
    // Build render method lines without backtick-inside-backtick issues
    const renderLines = [
        '    pub fn render(&self) -> String {',
        '        format!(',
        `            "<${tag} class=\\"${owClass}${variantClass}\\"${typeAttr}>{}</${tag}>",`,
        `            ${selfLabel}`,
        '        )',
        '    }',
    ];
    methods.push(renderLines.join('\n'));
    return methods.join('\n\n');
}
function generateTitan(ir) {
    const node = ir.rootWidget;
    const name = ir.name.replace(/[^A-Za-z0-9]/g, '') || 'Widget';
    const capitalName = name.charAt(0).toUpperCase() + name.slice(1);
    const structName = kindToTitanStruct(node.kind);
    const fields = renderStructFields(node);
    const implMethods = renderImpl(node, structName);
    // Pre-compute nested-template values to avoid backtick-inside-backtick in template literals
    const labelLiteral = node.label !== undefined ? ('"' + (node.label ?? name) + '"') : '';
    const testLabelLiteral = node.label !== undefined ? '"Test"' : '';
    const variantLine = (node.kind === 'button' || node.kind === 'toggle')
        ? 'self.variant = variant;'
        : '// variant not applicable';
    const sizeLine = node.size !== undefined ? 'self.size = size;' : '// size already set';
    const disabledLine = node.disabled !== undefined ? 'self.disabled = disabled;' : '// disabled flag';
    const checkedAssert = (node.kind === 'toggle' || node.kind === 'checkbox')
        ? 'assert!(!widget.checked, "Should be unchecked by default");'
        : '// no checked state';
    const disabledAssert = node.disabled !== undefined
        ? 'assert!(!widget.disabled, "Should not be disabled by default");'
        : '// no disabled state';
    return [
        `// ${capitalName} — Titan OW Widget Runtime`,
        `// Converted from ${ir.sourceLanguage} by Omnisystem Widget Converter`,
        `// Confidence: ${ir.confidence}`,
        '',
        'use crate::stdlib::WidgetIR::{Widget, WidgetVariant, WidgetSize};',
        '',
        `pub struct ${capitalName} {`,
        fields,
        '}',
        '',
        `impl Widget for ${capitalName} {`,
        implMethods,
        '}',
        '',
        `impl ${capitalName} {`,
        `    pub fn with_variant(mut self, variant: WidgetVariant) -> Self {`,
        `        ${variantLine}`,
        `        self`,
        `    }`,
        '',
        `    pub fn with_size(mut self, size: WidgetSize) -> Self {`,
        `        ${sizeLine}`,
        `        self`,
        `    }`,
        '',
        `    pub fn disabled(mut self, disabled: bool) -> Self {`,
        `        ${disabledLine}`,
        `        self`,
        `    }`,
        '}',
        '',
        '#[cfg(test)]',
        'mod tests {',
        '    use super::*;',
        '',
        '    #[test]',
        `    fn test_${capitalName.toLowerCase()}_renders() {`,
        `        let widget = ${capitalName}::new(${labelLiteral});`,
        '        let html = widget.render();',
        `        assert!(html.contains("ow-${node.kind}"), "Should contain OW class");`,
        '        assert!(!html.is_empty(), "Rendered HTML should not be empty");',
        '    }',
        '',
        '    #[test]',
        `    fn test_${capitalName.toLowerCase()}_default_state() {`,
        `        let widget = ${capitalName}::new(${testLabelLiteral});`,
        `        ${disabledAssert}`,
        `        ${checkedAssert}`,
        '    }',
        '}',
        '',
    ].join('\n');
}
//# sourceMappingURL=TitanGenerator.js.map