/**
 * OmniWidgetLibrary — TypeScript Widget Type System
 * Type definitions, registry, and interfaces for the Modular UI Widget System.
 * TypeScript is used here only for VS Code extension infrastructure.
 */
export type OmniThemeId = 'omni-dark' | 'omni-light' | 'omni-neon' | 'omni-forest' | 'omni-aurora' | 'omni-sunset';
export interface OmniTheme {
    id: OmniThemeId;
    label: string;
    swatch: string;
    desc: string;
    tokens: ThemeTokens;
}
export interface ThemeTokens {
    bg: string;
    bgCard: string;
    bgRaise: string;
    glass: string;
    overlay: string;
    accent: string;
    accentDim: string;
    accentGlow: string;
    success: string;
    warning: string;
    danger: string;
    purple: string;
    text: string;
    textDim: string;
    textMuted: string;
    border: string;
    borderFocus: string;
    shadow: string;
    shadowGlow: string;
}
export declare const OMNI_THEMES: Record<OmniThemeId, OmniTheme>;
export type WidgetCategory = 'Buttons' | 'Inputs' | 'Cards' | 'Navigation' | 'Feedback' | 'Data' | 'Overlays' | 'Special';
export interface WidgetDescriptor {
    id: string;
    cat: WidgetCategory;
    label: string;
    desc: string;
    cssClass?: string;
    variants?: string[];
    sizes?: string[];
    previewHtml?: string;
}
export interface BtnProps {
    label?: string;
    text?: string;
    icon?: string;
    variant?: 'primary' | 'solid' | 'ghost' | 'link' | 'danger' | 'success' | 'warning' | 'purple' | 'icon';
    size?: '2xs' | 'xs' | 'sm' | 'md' | 'lg' | 'xl';
    disabled?: boolean;
    title?: string;
    cls?: string;
    ripple?: boolean;
    onclick?: (e: MouseEvent) => void;
}
export interface InputProps {
    id?: string;
    type?: string;
    placeholder?: string;
    value?: string;
    readonly?: boolean;
    disabled?: boolean;
    maxlength?: number;
    size?: 'sm' | 'md' | 'lg';
    mono?: boolean;
    label?: string;
    hint?: string;
    error?: string;
    required?: boolean;
    oninput?: (e: Event) => void;
    onchange?: (e: Event) => void;
    onenter?: (e: KeyboardEvent) => void;
}
export interface CardProps {
    title?: string;
    subtitle?: string;
    icon?: string;
    body?: string | HTMLElement;
    html?: string;
    footer?: string | HTMLElement;
    actions?: HTMLElement;
    variant?: 'glass' | 'flat' | 'glow';
    interactive?: boolean;
    cls?: string;
    onclick?: (e: MouseEvent) => void;
}
export interface StatCardProps {
    value: number | string;
    label?: string;
    delta?: number;
    deltaUnit?: string;
    bar?: number;
    color?: string;
    cls?: string;
}
export interface BadgeProps {
    text: string;
    variant?: 'success' | 'warning' | 'danger' | 'info' | 'muted';
    pulse?: boolean;
}
export interface ProgressProps {
    value: number;
    label?: string;
    showPct?: boolean;
    variant?: 'success' | 'warning' | 'danger';
    size?: 'sm' | 'md' | 'lg';
    striped?: boolean;
    color?: string;
}
export interface TabItem {
    label: string;
    count?: number;
    content?: HTMLElement;
    html?: string;
}
export interface TabsProps {
    tabs: TabItem[];
    pill?: boolean;
    onchange?: (index: number, tab: TabItem) => void;
}
export interface ToastProps {
    msg: string;
    title?: string;
    variant?: 'success' | 'warning' | 'danger' | 'info';
    icon?: string;
    duration?: number;
}
export interface ModalProps {
    title: string;
    body?: string | HTMLElement;
    html?: string;
    footer?: string | HTMLElement;
    buttons?: BtnProps[];
    size?: 'sm' | 'lg' | 'xl' | 'full';
    closeOnBack?: boolean;
    onclose?: () => void;
}
export interface HealthRingProps {
    size?: number;
    strokeW?: number;
    color?: string;
    unit?: string;
}
export interface MetricProps {
    value: number | string;
    unit?: string;
    label?: string;
    trend?: number;
    trendUnit?: string;
    color?: string;
}
export interface TableColumn {
    key: string;
    label?: string;
    cls?: string;
    render?: (value: unknown, row: Record<string, unknown>, index: number) => string;
}
export interface TableProps {
    cols: TableColumn[];
    rows: Record<string, unknown>[];
    compact?: boolean;
    mono?: boolean;
    sortable?: boolean;
    empty?: string;
    onsort?: (key: string) => void;
    onrow?: (row: Record<string, unknown>, index: number) => void;
}
export interface NavItem {
    label: string;
    icon?: string;
    active?: boolean;
    badge?: string | number;
    onclick?: () => void;
}
export interface SideNavSection {
    label?: string;
    sep?: boolean;
    items: NavItem[];
}
export interface SideNavProps {
    sections?: SideNavSection[];
    items?: NavItem[];
    label?: string;
}
export interface DropdownItem {
    label?: string;
    icon?: string;
    key?: string;
    active?: boolean;
    danger?: boolean;
    cls?: string;
    sep?: boolean;
    onclick?: () => void;
}
export interface DropdownProps {
    label?: string;
    title?: string;
    items: DropdownItem[];
    trigger?: HTMLElement;
    btnOpts?: BtnProps;
}
export interface ChipProps {
    label: string;
    icon?: string;
    active?: boolean;
    cls?: string;
    onclick?: (e: MouseEvent) => void;
    onclose?: (el: HTMLElement) => void;
}
export interface EmptyProps {
    icon?: string;
    title?: string;
    desc?: string;
    action?: BtnProps;
}
export interface WidgetFactory<TProps = unknown> {
    (props: TProps): HTMLElement;
}
export declare class OmniWidgetRegistry {
    private static _db;
    private static _factories;
    static register(descriptor: WidgetDescriptor, factory?: WidgetFactory): void;
    static get(id: string): WidgetDescriptor | undefined;
    static getFactory(id: string): WidgetFactory | undefined;
    static all(): WidgetDescriptor[];
    static byCategory(cat: WidgetCategory): WidgetDescriptor[];
    static categories(): WidgetCategory[];
    static search(query: string): WidgetDescriptor[];
    static count(): number;
}
export interface OmniThemeChangeEvent extends CustomEvent {
    detail: {
        theme: OmniThemeId;
        prev: OmniThemeId;
    };
}
declare global {
    interface HTMLElementEventMap {
        'ow-theme-change': OmniThemeChangeEvent;
    }
}
export interface OWNamespace {
    _theme: OmniThemeId;
    widgetDB: WidgetDescriptor[];
    themes: OmniTheme[];
    switchTheme(themeId: OmniThemeId): void;
    loadTheme(): void;
    btn(opts: BtnProps): HTMLElement;
    btnGroup(buttons: BtnProps[]): HTMLElement;
    field(opts: InputProps): HTMLElement;
    input(opts: InputProps): HTMLInputElement;
    searchInput(opts: Partial<InputProps>): HTMLElement;
    select(opts: {
        id?: string;
        disabled?: boolean;
        options: Array<{
            value: string;
            label?: string;
        } | string>;
        value?: string;
        onchange?: (e: Event) => void;
    }): HTMLSelectElement;
    toggle(opts: {
        id?: string;
        label?: string;
        on?: boolean;
        size?: string;
        onchange?: (isOn: boolean) => void;
    }): HTMLElement;
    checkbox(opts: {
        id?: string;
        label?: string;
        checked?: boolean;
        disabled?: boolean;
        onchange?: (e: Event) => void;
    }): HTMLElement;
    slider(opts: {
        min?: number;
        max?: number;
        value?: number;
        labels?: [string, string];
        oninput?: (v: number) => void;
    }): HTMLElement;
    card(opts: CardProps): HTMLElement;
    statCard(opts: StatCardProps): HTMLElement;
    actionCard(opts: {
        title: string;
        desc?: string;
        icon?: string;
        cls?: string;
        onclick?: (e: MouseEvent) => void;
    }): HTMLElement;
    tabs(opts: TabsProps): HTMLElement;
    breadcrumbs(items: Array<{
        label: string;
        href?: string;
        onclick?: () => void;
    } | string>): HTMLElement;
    sideNav(opts: SideNavProps): HTMLElement;
    pagination(opts: {
        page: number;
        total: number;
        onpage: (p: number) => void;
    }): HTMLElement;
    badge(text: string, variant?: string, pulse?: boolean): HTMLElement;
    chip(opts: string | ChipProps): HTMLElement;
    progress(opts: number | ProgressProps): HTMLElement;
    spinner(size?: string): HTMLElement;
    alert(opts: string | {
        msg: string;
        title?: string;
        icon?: string;
        variant?: string;
        closable?: boolean;
    }): HTMLElement;
    toast(opts: string | ToastProps): HTMLElement;
    skeleton(opts?: {
        type?: 'card' | 'list' | 'text';
        rows?: number;
    }): HTMLElement;
    table(opts: TableProps): HTMLElement;
    code(opts: string | {
        code: string;
        lang?: string;
    }): HTMLElement;
    metric(opts: MetricProps): HTMLElement;
    sparkline(values: number[], opts?: {
        color?: string;
    }): HTMLElement;
    modal(opts: ModalProps): HTMLElement & {
        close(): void;
    };
    dropdown(opts: DropdownProps): HTMLElement;
    healthRing(value: number, opts?: HealthRingProps): HTMLElement;
    themePicker(opts?: {
        onchange?: (themeId: OmniThemeId) => void;
    }): HTMLElement;
    empty(opts: string | EmptyProps): HTMLElement;
    kbd(keys: string | string[]): HTMLElement;
    widgetBrowser(opts?: {
        onselect?: (w: WidgetDescriptor) => void;
    }): HTMLElement;
    registerWidget(def: WidgetDescriptor): void;
    el(tag: string, cls?: string, attrs?: Record<string, string>): HTMLElement;
}
declare global {
    const OW: OWNamespace;
}
//# sourceMappingURL=OmniWidgetLibrary.d.ts.map