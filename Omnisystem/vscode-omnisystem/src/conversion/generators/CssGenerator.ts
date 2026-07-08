// CSS Generator — converts WidgetIR to OW-compatible CSS class definitions
import { WidgetIR, WidgetNode, WidgetKind, WidgetStyle } from '../WidgetIR';

const OW_VARS = {
    bg:         'var(--ow-bg)',
    bgCard:     'var(--ow-bg-card)',
    bgRaise:    'var(--ow-bg-raise)',
    accent:     'var(--ow-accent)',
    accentDim:  'var(--ow-accent-dim)',
    text:       'var(--ow-text)',
    textDim:    'var(--ow-text-dim)',
    textMuted:  'var(--ow-text-muted)',
    border:     'var(--ow-border)',
    borderFocus:'var(--ow-border-focus)',
    success:    'var(--ow-success)',
    warning:    'var(--ow-warning)',
    danger:     'var(--ow-danger)',
    shadow:     'var(--ow-shadow-glow)',
    rSm:        'var(--ow-r-sm)',
    rMd:        'var(--ow-r-md)',
    rLg:        'var(--ow-r-lg)',
    rFull:      'var(--ow-r-full)',
    fontSans:   'var(--ow-font-sans)',
    fontMono:   'var(--ow-font-mono)',
    spaceSm:    'var(--ow-space-sm)',
    spaceMd:    'var(--ow-space-md)',
    ease:       'var(--ow-ease)',
    duration:   'var(--ow-duration)',
};

function baseStyles(kind: WidgetKind, name: string): string {
    switch (kind) {
        case 'button':
            return `.${name} {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 8px 18px;
  background: ${OW_VARS.accent};
  color: ${OW_VARS.bg};
  border: none;
  border-radius: ${OW_VARS.rMd};
  font-size: 13px;
  font-weight: 700;
  font-family: ${OW_VARS.fontSans};
  letter-spacing: 0.3px;
  cursor: pointer;
  transition: all ${OW_VARS.duration} ${OW_VARS.ease};
  white-space: nowrap;
  user-select: none;
}

.${name}:hover {
  filter: brightness(1.12);
  transform: translateY(-1px);
  box-shadow: ${OW_VARS.shadow};
}

.${name}:active {
  transform: translateY(0);
  filter: brightness(0.95);
}

.${name}:focus-visible {
  outline: 2px solid ${OW_VARS.borderFocus};
  outline-offset: 2px;
}

.${name}:disabled,
.${name}[disabled] {
  opacity: 0.45;
  cursor: not-allowed;
  transform: none;
  pointer-events: none;
}

/* Variants */
.${name}--primary   { background: ${OW_VARS.accent}; color: ${OW_VARS.bg}; }
.${name}--secondary { background: transparent; color: ${OW_VARS.accent}; border: 1px solid ${OW_VARS.border}; }
.${name}--danger    { background: ${OW_VARS.danger}; color: #fff; }
.${name}--warning   { background: ${OW_VARS.warning}; color: ${OW_VARS.bg}; }
.${name}--success   { background: ${OW_VARS.success}; color: ${OW_VARS.bg}; }
.${name}--ghost     { background: transparent; color: ${OW_VARS.textDim}; border: 1px solid transparent; }
.${name}--ghost:hover { background: ${OW_VARS.bgRaise}; color: ${OW_VARS.text}; }

/* Sizes */
.${name}--xs { padding: 4px 10px; font-size: 11px; }
.${name}--sm { padding: 6px 14px; font-size: 12px; }
.${name}--md { padding: 8px 18px; font-size: 13px; }
.${name}--lg { padding: 10px 24px; font-size: 15px; }
.${name}--xl { padding: 14px 32px; font-size: 17px; }`;

        case 'input':
        case 'textarea':
            return `.${name} {
  display: block;
  width: 100%;
  padding: 9px 12px;
  background: ${OW_VARS.bgCard};
  color: ${OW_VARS.text};
  border: 1px solid ${OW_VARS.border};
  border-radius: ${OW_VARS.rSm};
  font-size: 13px;
  font-family: ${OW_VARS.fontSans};
  transition: border-color ${OW_VARS.duration} ${OW_VARS.ease}, box-shadow ${OW_VARS.duration} ${OW_VARS.ease};
  outline: none;
  ${kind === 'textarea' ? 'resize: vertical;\n  min-height: 80px;' : ''}
}

.${name}::placeholder {
  color: ${OW_VARS.textMuted};
}

.${name}:focus {
  border-color: ${OW_VARS.borderFocus};
  box-shadow: 0 0 0 3px ${OW_VARS.accentDim};
}

.${name}:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  background: ${OW_VARS.bg};
}`;

        case 'toggle':
            return `.${name} {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  user-select: none;
}

.${name}__track {
  position: relative;
  width: 40px;
  height: 22px;
  background: ${OW_VARS.border};
  border-radius: ${OW_VARS.rFull};
  transition: background ${OW_VARS.duration} ${OW_VARS.ease};
}

.${name}--on .${name}__track {
  background: ${OW_VARS.accent};
}

.${name}__knob {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 16px;
  height: 16px;
  background: #fff;
  border-radius: 50%;
  transition: transform ${OW_VARS.duration} ${OW_VARS.ease};
  box-shadow: 0 1px 4px rgba(0,0,0,0.3);
}

.${name}--on .${name}__knob {
  transform: translateX(18px);
}

.${name}:focus-visible .${name}__track {
  outline: 2px solid ${OW_VARS.borderFocus};
  outline-offset: 2px;
}`;

        case 'card':
            return `.${name} {
  background: ${OW_VARS.bgCard};
  border: 1px solid ${OW_VARS.border};
  border-radius: ${OW_VARS.rLg};
  padding: 20px;
  transition: border-color ${OW_VARS.duration} ${OW_VARS.ease}, box-shadow ${OW_VARS.duration} ${OW_VARS.ease};
}

.${name}:hover {
  border-color: ${OW_VARS.borderFocus};
  box-shadow: ${OW_VARS.shadow};
}

.${name}__title {
  color: ${OW_VARS.accent};
  font-size: 15px;
  font-weight: 600;
  margin: 0 0 8px;
}

.${name}__body {
  color: ${OW_VARS.textDim};
  font-size: 13px;
  line-height: 1.6;
}

.${name}__footer {
  margin-top: 16px;
  display: flex;
  gap: 8px;
  border-top: 1px solid ${OW_VARS.border};
  padding-top: 12px;
}`;

        case 'badge':
        case 'chip':
            return `.${name} {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 10px;
  background: ${OW_VARS.accentDim};
  color: ${OW_VARS.accent};
  border: 1px solid ${OW_VARS.border};
  border-radius: ${OW_VARS.rFull};
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.5px;
  white-space: nowrap;
}`;

        case 'progress':
            return `.${name} {
  width: 100%;
  height: 6px;
  background: ${OW_VARS.border};
  border-radius: ${OW_VARS.rFull};
  overflow: hidden;
}

.${name}__fill {
  height: 100%;
  background: ${OW_VARS.accent};
  border-radius: ${OW_VARS.rFull};
  transition: width 0.4s ${OW_VARS.ease};
}`;

        case 'modal':
            return `.${name}__backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.${name} {
  background: ${OW_VARS.bgCard};
  border: 1px solid ${OW_VARS.border};
  border-radius: ${OW_VARS.rLg};
  padding: 28px;
  min-width: 340px;
  max-width: min(90vw, 600px);
  max-height: 85vh;
  overflow-y: auto;
  box-shadow: 0 24px 80px rgba(0,0,0,0.4);
}

.${name}__title {
  color: ${OW_VARS.accent};
  font-size: 18px;
  font-weight: 700;
  margin: 0 0 16px;
}

.${name}__body { color: ${OW_VARS.text}; line-height: 1.6; }

.${name}__footer {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}`;

        case 'alert':
            return `.${name} {
  display: flex;
  gap: 12px;
  padding: 14px 16px;
  background: ${OW_VARS.bgCard};
  border: 1px solid ${OW_VARS.border};
  border-radius: ${OW_VARS.rMd};
  border-left: 4px solid ${OW_VARS.accent};
  color: ${OW_VARS.text};
  font-size: 13px;
  line-height: 1.5;
}

.${name}--success { border-left-color: ${OW_VARS.success}; }
.${name}--warning { border-left-color: ${OW_VARS.warning}; }
.${name}--danger  { border-left-color: ${OW_VARS.danger}; }`;

        case 'select':
            return `.${name} {
  display: block;
  width: 100%;
  padding: 9px 36px 9px 12px;
  background: ${OW_VARS.bgCard};
  color: ${OW_VARS.text};
  border: 1px solid ${OW_VARS.border};
  border-radius: ${OW_VARS.rSm};
  font-size: 13px;
  font-family: ${OW_VARS.fontSans};
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='8' viewBox='0 0 12 8'%3E%3Cpath d='M1 1l5 5 5-5' stroke='%2300D4FF' stroke-width='1.5' fill='none'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 12px center;
  outline: none;
}

.${name}:focus {
  border-color: ${OW_VARS.borderFocus};
  box-shadow: 0 0 0 3px ${OW_VARS.accentDim};
}`;

        default:
            return `.${name} {
  background: ${OW_VARS.bgCard};
  color: ${OW_VARS.text};
  border: 1px solid ${OW_VARS.border};
  border-radius: ${OW_VARS.rMd};
  padding: ${OW_VARS.spaceMd};
  font-family: ${OW_VARS.fontSans};
  transition: all ${OW_VARS.duration} ${OW_VARS.ease};
}

.${name}:focus-visible {
  outline: 2px solid ${OW_VARS.borderFocus};
  outline-offset: 2px;
}`;
    }
}

export function generateCss(ir: WidgetIR): string {
    const node = ir.rootWidget;
    const rawName = ir.name.replace(/[^A-Za-z0-9]/g, '-').replace(/-+/g, '-').replace(/^-|-$/g, '').toLowerCase() || 'widget';

    const themeBlock = `/* ── Theme compatibility ────────────────────────────────────────────────────
 * All colors reference OW CSS custom properties.
 * They update automatically when [data-theme="omni-*"] changes.
 * Supported: omni-dark, omni-light, omni-neon, omni-forest, omni-aurora, omni-sunset
 */`;

    return `/* ${rawName} — OW CSS Widget
 * Converted from ${ir.sourceLanguage} by Omnisystem Widget Converter
 * Confidence: ${ir.confidence}
 *
 * Usage: <link rel="stylesheet" href="omni-widgets.css" />
 * Then add class="${rawName}" to your HTML element.
 */

${themeBlock}

${baseStyles(node.kind, rawName)}

/* ── Responsive ─────────────────────────────────────────────────────────────── */
@media (max-width: 640px) {
  .${rawName} {
    width: 100%;
    box-sizing: border-box;
  }
}
`;
}
