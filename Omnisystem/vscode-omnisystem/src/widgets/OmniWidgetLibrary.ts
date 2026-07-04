/// <reference lib="dom" />
/**
 * OmniWidgetLibrary — TypeScript Widget Type System
 * Type definitions, registry, and interfaces for the Modular UI Widget System.
 * TypeScript is used here only for VS Code extension infrastructure.
 */

// ── Theme types ──────────────────────────────────────────────────────────────

export type OmniThemeId =
  | 'omni-dark'
  | 'omni-light'
  | 'omni-neon'
  | 'omni-forest'
  | 'omni-aurora'
  | 'omni-sunset';

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

export const OMNI_THEMES: Record<OmniThemeId, OmniTheme> = {
  'omni-dark': {
    id: 'omni-dark', label: 'OmniDark', swatch: 'linear-gradient(135deg,#050D1A,#00D4FF)',
    desc: 'Deep space blue',
    tokens: { bg:'#050D1A', bgCard:'rgba(10,20,42,0.86)', bgRaise:'rgba(0,20,50,0.52)', glass:'rgba(8,18,36,0.72)', overlay:'rgba(0,0,0,0.86)', accent:'#00D4FF', accentDim:'rgba(0,212,255,0.13)', accentGlow:'rgba(0,212,255,0.24)', success:'#00FF88', warning:'#FFB800', danger:'#FF4466', purple:'#BF88FF', text:'#E8F4FF', textDim:'rgba(232,244,255,0.52)', textMuted:'rgba(232,244,255,0.28)', border:'rgba(0,212,255,0.18)', borderFocus:'rgba(0,212,255,0.62)', shadow:'0 3px 16px rgba(0,212,255,0.08)', shadowGlow:'0 0 22px rgba(0,212,255,0.28)' },
  },
  'omni-light': {
    id: 'omni-light', label: 'OmniLight', swatch: 'linear-gradient(135deg,#EEF2FF,#1A6CF0)',
    desc: 'Clean & bright',
    tokens: { bg:'#EEF2FF', bgCard:'rgba(255,255,255,0.92)', bgRaise:'rgba(220,230,255,0.62)', glass:'rgba(240,245,255,0.80)', overlay:'rgba(0,10,40,0.62)', accent:'#1A6CF0', accentDim:'rgba(26,108,240,0.12)', accentGlow:'rgba(26,108,240,0.22)', success:'#0DA84E', warning:'#D07A00', danger:'#D62B4A', purple:'#7C3AED', text:'#0A1230', textDim:'rgba(10,18,48,0.55)', textMuted:'rgba(10,18,48,0.32)', border:'rgba(26,108,240,0.22)', borderFocus:'rgba(26,108,240,0.72)', shadow:'0 3px 16px rgba(0,0,80,0.08)', shadowGlow:'0 0 22px rgba(26,108,240,0.22)' },
  },
  'omni-neon': {
    id: 'omni-neon', label: 'OmniNeon', swatch: 'linear-gradient(135deg,#000000,#00FF41)',
    desc: 'Terminal green',
    tokens: { bg:'#000000', bgCard:'rgba(0,18,4,0.90)', bgRaise:'rgba(0,30,8,0.64)', glass:'rgba(0,12,2,0.82)', overlay:'rgba(0,0,0,0.92)', accent:'#00FF41', accentDim:'rgba(0,255,65,0.12)', accentGlow:'rgba(0,255,65,0.28)', success:'#00FF41', warning:'#FFFF00', danger:'#FF003C', purple:'#BF00FF', text:'#CCFFDD', textDim:'rgba(200,255,220,0.52)', textMuted:'rgba(200,255,220,0.28)', border:'rgba(0,255,65,0.22)', borderFocus:'rgba(0,255,65,0.72)', shadow:'0 3px 16px rgba(0,255,65,0.06)', shadowGlow:'0 0 22px rgba(0,255,65,0.32)' },
  },
  'omni-forest': {
    id: 'omni-forest', label: 'OmniForest', swatch: 'linear-gradient(135deg,#050F07,#3CFF7E)',
    desc: 'Deep forest',
    tokens: { bg:'#050F07', bgCard:'rgba(8,22,10,0.88)', bgRaise:'rgba(12,32,15,0.58)', glass:'rgba(6,16,8,0.76)', overlay:'rgba(0,5,2,0.88)', accent:'#3CFF7E', accentDim:'rgba(60,255,126,0.12)', accentGlow:'rgba(60,255,126,0.24)', success:'#7EFF58', warning:'#AAFF00', danger:'#FF4D4D', purple:'#99AAFF', text:'#DDFAE6', textDim:'rgba(220,250,230,0.52)', textMuted:'rgba(220,250,230,0.28)', border:'rgba(60,255,126,0.18)', borderFocus:'rgba(60,255,126,0.62)', shadow:'0 3px 16px rgba(60,255,126,0.06)', shadowGlow:'0 0 22px rgba(60,255,126,0.26)' },
  },
  'omni-aurora': {
    id: 'omni-aurora', label: 'OmniAurora', swatch: 'linear-gradient(135deg,#0B071E,#C07AFF)',
    desc: 'Violet aurora',
    tokens: { bg:'#0B071E', bgCard:'rgba(18,12,38,0.88)', bgRaise:'rgba(28,16,52,0.54)', glass:'rgba(14,10,28,0.76)', overlay:'rgba(0,0,10,0.88)', accent:'#C07AFF', accentDim:'rgba(192,122,255,0.13)', accentGlow:'rgba(192,122,255,0.26)', success:'#4AFFC8', warning:'#FFD060', danger:'#FF4488', purple:'#C07AFF', text:'#F0EAFF', textDim:'rgba(240,234,255,0.52)', textMuted:'rgba(240,234,255,0.28)', border:'rgba(192,122,255,0.18)', borderFocus:'rgba(192,122,255,0.62)', shadow:'0 3px 16px rgba(192,122,255,0.08)', shadowGlow:'0 0 22px rgba(192,122,255,0.30)' },
  },
  'omni-sunset': {
    id: 'omni-sunset', label: 'OmniSunset', swatch: 'linear-gradient(135deg,#160700,#FF8C00)',
    desc: 'Warm sunset',
    tokens: { bg:'#160700', bgCard:'rgba(30,12,0,0.88)', bgRaise:'rgba(44,16,0,0.54)', glass:'rgba(22,8,0,0.76)', overlay:'rgba(10,2,0,0.88)', accent:'#FF8C00', accentDim:'rgba(255,140,0,0.13)', accentGlow:'rgba(255,140,0,0.26)', success:'#AAFF44', warning:'#FFD700', danger:'#FF2244', purple:'#FF88CC', text:'#FFF0E0', textDim:'rgba(255,240,224,0.52)', textMuted:'rgba(255,240,224,0.28)', border:'rgba(255,140,0,0.18)', borderFocus:'rgba(255,140,0,0.62)', shadow:'0 3px 16px rgba(255,140,0,0.08)', shadowGlow:'0 0 22px rgba(255,140,0,0.28)' },
  },
};

// ── Widget category ───────────────────────────────────────────────────────────

export type WidgetCategory =
  | 'Buttons'
  | 'Inputs'
  | 'Cards'
  | 'Navigation'
  | 'Feedback'
  | 'Data'
  | 'Overlays'
  | 'Special';

// ── Widget descriptor ─────────────────────────────────────────────────────────

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

// ── Widget props ──────────────────────────────────────────────────────────────

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

// ── Widget Registry ───────────────────────────────────────────────────────────

export interface WidgetFactory<TProps = unknown> {
  (props: TProps): HTMLElement;
}

export class OmniWidgetRegistry {
  private static _db = new Map<string, WidgetDescriptor>();
  private static _factories = new Map<string, WidgetFactory>();

  static register(descriptor: WidgetDescriptor, factory?: WidgetFactory): void {
    this._db.set(descriptor.id, descriptor);
    if (factory) this._factories.set(descriptor.id, factory);
  }

  static get(id: string): WidgetDescriptor | undefined {
    return this._db.get(id);
  }

  static getFactory(id: string): WidgetFactory | undefined {
    return this._factories.get(id);
  }

  static all(): WidgetDescriptor[] {
    return [...this._db.values()];
  }

  static byCategory(cat: WidgetCategory): WidgetDescriptor[] {
    return this.all().filter(w => w.cat === cat);
  }

  static categories(): WidgetCategory[] {
    return [...new Set(this.all().map(w => w.cat))] as WidgetCategory[];
  }

  static search(query: string): WidgetDescriptor[] {
    const q = query.toLowerCase();
    return this.all().filter(w =>
      w.label.toLowerCase().includes(q) ||
      w.desc.toLowerCase().includes(q) ||
      w.cat.toLowerCase().includes(q)
    );
  }

  static count(): number {
    return this._db.size;
  }
}

// ── Theme event ───────────────────────────────────────────────────────────────

export interface OmniThemeChangeEvent extends CustomEvent {
  detail: { theme: OmniThemeId; prev: OmniThemeId };
}

declare global {
  interface HTMLElementEventMap {
    'ow-theme-change': OmniThemeChangeEvent;
  }
}

// ── OW global (from omni-widgets.js) ─────────────────────────────────────────

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
  select(opts: { id?: string; disabled?: boolean; options: Array<{value: string; label?: string} | string>; value?: string; onchange?: (e: Event) => void }): HTMLSelectElement;
  toggle(opts: { id?: string; label?: string; on?: boolean; size?: string; onchange?: (isOn: boolean) => void }): HTMLElement;
  checkbox(opts: { id?: string; label?: string; checked?: boolean; disabled?: boolean; onchange?: (e: Event) => void }): HTMLElement;
  slider(opts: { min?: number; max?: number; value?: number; labels?: [string, string]; oninput?: (v: number) => void }): HTMLElement;
  card(opts: CardProps): HTMLElement;
  statCard(opts: StatCardProps): HTMLElement;
  actionCard(opts: { title: string; desc?: string; icon?: string; cls?: string; onclick?: (e: MouseEvent) => void }): HTMLElement;
  tabs(opts: TabsProps): HTMLElement;
  breadcrumbs(items: Array<{label: string; href?: string; onclick?: () => void} | string>): HTMLElement;
  sideNav(opts: SideNavProps): HTMLElement;
  pagination(opts: { page: number; total: number; onpage: (p: number) => void }): HTMLElement;
  badge(text: string, variant?: string, pulse?: boolean): HTMLElement;
  chip(opts: string | ChipProps): HTMLElement;
  progress(opts: number | ProgressProps): HTMLElement;
  spinner(size?: string): HTMLElement;
  alert(opts: string | { msg: string; title?: string; icon?: string; variant?: string; closable?: boolean }): HTMLElement;
  toast(opts: string | ToastProps): HTMLElement;
  skeleton(opts?: { type?: 'card' | 'list' | 'text'; rows?: number }): HTMLElement;
  table(opts: TableProps): HTMLElement;
  code(opts: string | { code: string; lang?: string }): HTMLElement;
  metric(opts: MetricProps): HTMLElement;
  sparkline(values: number[], opts?: { color?: string }): HTMLElement;
  modal(opts: ModalProps): HTMLElement & { close(): void };
  dropdown(opts: DropdownProps): HTMLElement;
  healthRing(value: number, opts?: HealthRingProps): HTMLElement;
  themePicker(opts?: { onchange?: (themeId: OmniThemeId) => void }): HTMLElement;
  empty(opts: string | EmptyProps): HTMLElement;
  kbd(keys: string | string[]): HTMLElement;
  widgetBrowser(opts?: { onselect?: (w: WidgetDescriptor) => void }): HTMLElement;
  registerWidget(def: WidgetDescriptor): void;
  el(tag: string, cls?: string, attrs?: Record<string, string>): HTMLElement;
}

declare global {
  const OW: OWNamespace;
}
