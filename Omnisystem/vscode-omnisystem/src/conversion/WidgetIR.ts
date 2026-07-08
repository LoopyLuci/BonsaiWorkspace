// Universal Widget Intermediate Representation
// Shared types used by all parsers and generators

export type WidgetKind =
    | 'button' | 'input' | 'textarea' | 'checkbox' | 'radio' | 'toggle'
    | 'select' | 'slider' | 'card' | 'panel' | 'modal' | 'drawer'
    | 'tab' | 'tabgroup' | 'accordion' | 'list' | 'listitem'
    | 'table' | 'grid' | 'form' | 'label' | 'badge' | 'tag'
    | 'progress' | 'spinner' | 'icon' | 'image' | 'avatar'
    | 'divider' | 'spacer' | 'container' | 'navbar' | 'sidebar'
    | 'tooltip' | 'popover' | 'toast' | 'alert' | 'banner'
    | 'chart' | 'tree' | 'calendar' | 'datepicker' | 'colorpicker'
    | 'breadcrumb' | 'pagination' | 'stepper' | 'rating' | 'chip'
    | 'unknown';

export type WidgetVariant =
    | 'primary' | 'secondary' | 'danger' | 'warning' | 'success'
    | 'ghost' | 'outline' | 'link' | 'flat' | 'filled' | 'tonal';

export type WidgetSize = 'xs' | 'sm' | 'md' | 'lg' | 'xl';

export interface WidgetStyle {
    color?: string;
    background?: string;
    border?: string;
    borderRadius?: string;
    padding?: string;
    margin?: string;
    width?: string;
    height?: string;
    minWidth?: string;
    maxWidth?: string;
    fontSize?: string;
    fontWeight?: string;
    fontFamily?: string;
    display?: string;
    flexDirection?: string;
    gap?: string;
    alignItems?: string;
    justifyContent?: string;
    boxShadow?: string;
    opacity?: string;
    cursor?: string;
    overflow?: string;
    textAlign?: string;
    transition?: string;
    transform?: string;
    zIndex?: string;
    position?: string;
    top?: string;
    left?: string;
}

export interface WidgetEvent {
    name: string;       // onClick, onChange, onSubmit, onFocus, etc.
    handler: string;    // event handler body or function reference name
    params?: string[];  // parameter names
}

export interface WidgetProp {
    name: string;
    type: string;           // string, number, boolean, function, etc.
    value?: string;         // default or current value
    required?: boolean;
    description?: string;
}

export interface WidgetNode {
    id: string;
    kind: WidgetKind;
    name?: string;          // component/widget name
    label?: string;         // display text / aria-label
    placeholder?: string;
    value?: string;
    defaultValue?: string;
    disabled?: boolean;
    checked?: boolean;
    required?: boolean;
    multiple?: boolean;
    variant?: WidgetVariant;
    size?: WidgetSize;
    icon?: string;
    href?: string;
    style?: WidgetStyle;
    className?: string;
    props?: WidgetProp[];
    events?: WidgetEvent[];
    children?: WidgetNode[];
    options?: Array<{ value: string; label: string }>;
    slots?: Record<string, WidgetNode>;
    raw?: string;           // original source snippet (first 500 chars)
    meta?: Record<string, string>;
}

export interface WidgetIR {
    name: string;
    description?: string;
    rootWidget: WidgetNode;
    imports?: string[];
    dependencies?: string[];
    sourceLanguage: SourceLanguage;
    confidence: ConversionConfidence;
    notes?: string[];
}

export type SourceLanguage =
    | 'javascript' | 'typescript' | 'css' | 'tauri' | 'python'
    | 'vera' | 'nexus' | 'titan';

export type TargetLanguage =
    | 'vera' | 'nexus' | 'titan'
    | 'javascript' | 'typescript' | 'css' | 'tauri' | 'python';

export type ConversionConfidence = 'high' | 'medium' | 'low';

export interface ConversionResult {
    code: string;
    widgetType: WidgetKind;
    widgetName: string;
    confidence: ConversionConfidence;
    notes: string[];
    targetLanguage: TargetLanguage;
    fileExtension: string;
}

export function makeId(seed: string): string {
    return seed.toLowerCase().replace(/[^a-z0-9]/g, '_').replace(/_+/g, '_').replace(/^_|_$/g, '') || 'widget';
}

export const LANGUAGE_EXTENSIONS: Record<TargetLanguage, string> = {
    vera:       '.vera',
    nexus:      '.nexus',
    titan:      '.titan',
    javascript: '.js',
    typescript: '.ts',
    css:        '.css',
    tauri:      '.html',
    python:     '.py',
};

export const LANGUAGE_LABELS: Record<SourceLanguage | TargetLanguage, string> = {
    javascript: 'JavaScript',
    typescript: 'TypeScript',
    css:        'CSS',
    tauri:      'Tauri (HTML+JS)',
    python:     'Python GUI (Tkinter/PyQt)',
    vera:       'Vera (OW Component)',
    nexus:      'Nexus (OW Layout)',
    titan:      'Titan (OW Runtime)',
};
