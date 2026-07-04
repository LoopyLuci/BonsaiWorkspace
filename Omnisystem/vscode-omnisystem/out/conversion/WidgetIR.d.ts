export type WidgetKind = 'button' | 'input' | 'textarea' | 'checkbox' | 'radio' | 'toggle' | 'select' | 'slider' | 'card' | 'panel' | 'modal' | 'drawer' | 'tab' | 'tabgroup' | 'accordion' | 'list' | 'listitem' | 'table' | 'grid' | 'form' | 'label' | 'badge' | 'tag' | 'progress' | 'spinner' | 'icon' | 'image' | 'avatar' | 'divider' | 'spacer' | 'container' | 'navbar' | 'sidebar' | 'tooltip' | 'popover' | 'toast' | 'alert' | 'banner' | 'chart' | 'tree' | 'calendar' | 'datepicker' | 'colorpicker' | 'breadcrumb' | 'pagination' | 'stepper' | 'rating' | 'chip' | 'unknown';
export type WidgetVariant = 'primary' | 'secondary' | 'danger' | 'warning' | 'success' | 'ghost' | 'outline' | 'link' | 'flat' | 'filled' | 'tonal';
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
    name: string;
    handler: string;
    params?: string[];
}
export interface WidgetProp {
    name: string;
    type: string;
    value?: string;
    required?: boolean;
    description?: string;
}
export interface WidgetNode {
    id: string;
    kind: WidgetKind;
    name?: string;
    label?: string;
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
    options?: Array<{
        value: string;
        label: string;
    }>;
    slots?: Record<string, WidgetNode>;
    raw?: string;
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
export type SourceLanguage = 'javascript' | 'typescript' | 'css' | 'tauri' | 'python' | 'vera' | 'nexus' | 'titan';
export type TargetLanguage = 'vera' | 'nexus' | 'titan' | 'javascript' | 'typescript' | 'css' | 'tauri' | 'python';
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
export declare function makeId(seed: string): string;
export declare const LANGUAGE_EXTENSIONS: Record<TargetLanguage, string>;
export declare const LANGUAGE_LABELS: Record<SourceLanguage | TargetLanguage, string>;
//# sourceMappingURL=WidgetIR.d.ts.map