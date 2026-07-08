"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.parseCss = parseCss;
// CSS Parser — detects widget patterns from CSS class/component definitions
const WidgetIR_1 = require("../WidgetIR");
const CLASS_TO_KIND = [
    [/\bbtn\b|button/i, 'button'],
    [/\binput\b|text-field|textfield/i, 'input'],
    [/\btextarea\b/i, 'textarea'],
    [/\bcheckbox\b/i, 'checkbox'],
    [/\bradio\b/i, 'radio'],
    [/\btoggle\b|switch/i, 'toggle'],
    [/\bselect\b|dropdown/i, 'select'],
    [/\bslider\b|range/i, 'slider'],
    [/\bcard\b/i, 'card'],
    [/\bmodal\b|dialog/i, 'modal'],
    [/\bdrawer\b|sheet/i, 'drawer'],
    [/\btabs?\b|tabgroup/i, 'tabgroup'],
    [/\btab(?!le)\b/i, 'tab'],
    [/\blist(?!item)\b/i, 'list'],
    [/\blistitem\b|list-item/i, 'listitem'],
    [/\btable\b/i, 'table'],
    [/\bgrid\b/i, 'grid'],
    [/\bform\b/i, 'form'],
    [/\bbadge\b/i, 'badge'],
    [/\btag\b|chip\b/i, 'chip'],
    [/\bprogress\b/i, 'progress'],
    [/\bspinner\b|loader\b/i, 'spinner'],
    [/\bavatar\b/i, 'avatar'],
    [/\btooltip\b/i, 'tooltip'],
    [/\btoast\b|snackbar/i, 'toast'],
    [/\balert\b|notification/i, 'alert'],
    [/\bnav(?:bar)?\b/i, 'navbar'],
    [/\bsidebar\b/i, 'sidebar'],
    [/\bbreadcrumb\b/i, 'breadcrumb'],
    [/\bpagination\b/i, 'pagination'],
    [/\brating\b|star/i, 'rating'],
    [/\bdivider\b|separator/i, 'divider'],
    [/\bpanel\b|section\b/i, 'panel'],
    [/\bcontainer\b|wrapper\b/i, 'container'],
];
function parseCssRules(src) {
    const rules = [];
    // Match .className { ... } blocks
    const ruleRx = /([.#][\w-]+(?:\s*,\s*[.#][\w-]+)*)\s*\{([^}]*)\}/g;
    let m;
    while ((m = ruleRx.exec(src)) !== null) {
        const selector = m[1].trim();
        const body = m[2];
        const properties = {};
        const propRx = /([\w-]+)\s*:\s*([^;]+)\s*;/g;
        let pm;
        while ((pm = propRx.exec(body)) !== null) {
            properties[pm[1].trim()] = pm[2].trim();
        }
        // Use the first class name as the canonical name
        const firstClass = selector.split(',')[0].trim().replace(/^[.#]/, '');
        rules.push({ selector, className: firstClass, properties });
    }
    return rules;
}
function cssPropsToWidgetStyle(props) {
    const style = {};
    const map = {
        'color': 'color', 'background': 'background', 'background-color': 'background',
        'border': 'border', 'border-radius': 'borderRadius', 'border-color': 'border',
        'padding': 'padding', 'margin': 'margin',
        'width': 'width', 'height': 'height', 'min-width': 'minWidth', 'max-width': 'maxWidth',
        'font-size': 'fontSize', 'font-weight': 'fontWeight', 'font-family': 'fontFamily',
        'display': 'display', 'flex-direction': 'flexDirection', 'gap': 'gap',
        'align-items': 'alignItems', 'justify-content': 'justifyContent',
        'box-shadow': 'boxShadow', 'opacity': 'opacity', 'cursor': 'cursor',
        'overflow': 'overflow', 'text-align': 'textAlign', 'transition': 'transition',
        'z-index': 'zIndex', 'position': 'position',
    };
    for (const [cssProp, irProp] of Object.entries(map)) {
        if (props[cssProp]) {
            style[irProp] = props[cssProp];
        }
    }
    return style;
}
function classNameToKind(className) {
    for (const [rx, kind] of CLASS_TO_KIND) {
        if (rx.test(className)) {
            return kind;
        }
    }
    return 'unknown';
}
function classNameToHumanName(className) {
    return className
        .replace(/-/g, ' ')
        .replace(/_/g, ' ')
        .replace(/\b\w/g, c => c.toUpperCase())
        .trim() || 'Widget';
}
function parseCss(source) {
    const src = source.trim();
    const rules = parseCssRules(src);
    const notes = [];
    if (rules.length === 0) {
        return {
            name: 'Widget',
            rootWidget: { id: 'widget', kind: 'unknown' },
            sourceLanguage: 'css',
            confidence: 'low',
            notes: ['No CSS rules detected'],
        };
    }
    // Find the "main" rule (first non-modifier class, or the one with most properties)
    let mainRule = rules.reduce((best, r) => {
        const isModifier = /:hover|:focus|:active|::before|::after|:disabled/.test(r.selector);
        if (isModifier) {
            return best;
        }
        return Object.keys(r.properties).length > Object.keys(best.properties).length ? r : best;
    }, rules[0]);
    const kind = classNameToKind(mainRule.className);
    const name = classNameToHumanName(mainRule.className);
    const style = cssPropsToWidgetStyle(mainRule.properties);
    notes.push(`Detected ${rules.length} CSS rule(s)`);
    if (kind !== 'unknown') {
        notes.push(`Widget type inferred from class name: "${mainRule.className}"`);
    }
    else {
        notes.push('Widget type unknown — class name did not match a known widget pattern');
    }
    // Detect OW CSS variables
    const hasOwVars = src.includes('--ow-');
    if (hasOwVars) {
        notes.push('OW CSS custom properties detected — already OW-compatible');
    }
    // Detect states (hover/focus/active rules)
    const stateRules = rules.filter(r => /:hover|:focus|:active/.test(r.selector));
    if (stateRules.length > 0) {
        notes.push(`${stateRules.length} state variants found (hover/focus/active)`);
    }
    const confidence = kind !== 'unknown' ? 'high' : 'medium';
    const node = {
        id: (0, WidgetIR_1.makeId)(name),
        kind,
        name,
        className: mainRule.className,
        style,
    };
    return {
        name,
        rootWidget: node,
        sourceLanguage: 'css',
        confidence,
        notes,
    };
}
//# sourceMappingURL=CssParser.js.map