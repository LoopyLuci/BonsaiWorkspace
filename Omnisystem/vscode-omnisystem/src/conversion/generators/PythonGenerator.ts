// Python Generator — converts WidgetIR to Python GUI code (Tkinter + PyQt)
import { WidgetIR, WidgetNode, WidgetKind, WidgetEvent } from '../WidgetIR';

type PyFramework = 'tkinter' | 'pyqt';

function kindToTkWidget(kind: WidgetKind): string {
    const map: Partial<Record<WidgetKind, string>> = {
        button:    'tk.Button',
        input:     'ttk.Entry',
        textarea:  'tk.Text',
        checkbox:  'ttk.Checkbutton',
        radio:     'ttk.Radiobutton',
        toggle:    'ttk.Checkbutton',
        select:    'ttk.Combobox',
        slider:    'ttk.Scale',
        card:      'ttk.LabelFrame',
        panel:     'ttk.Frame',
        modal:     'tk.Toplevel',
        badge:     'ttk.Label',
        label:     'ttk.Label',
        progress:  'ttk.Progressbar',
        list:      'tk.Listbox',
        table:     'ttk.Treeview',
        form:      'ttk.Frame',
        navbar:    'tk.Menu',
        tabgroup:  'ttk.Notebook',
        divider:   'ttk.Separator',
        container: 'ttk.Frame',
        spinner:   'ttk.Progressbar',
        alert:     'tk.Toplevel',
    };
    return map[kind] ?? 'ttk.Frame';
}

function kindToQtWidget(kind: WidgetKind): string {
    const map: Partial<Record<WidgetKind, string>> = {
        button:    'QPushButton',
        input:     'QLineEdit',
        textarea:  'QTextEdit',
        checkbox:  'QCheckBox',
        radio:     'QRadioButton',
        toggle:    'QCheckBox',
        select:    'QComboBox',
        slider:    'QSlider',
        card:      'QGroupBox',
        panel:     'QFrame',
        modal:     'QDialog',
        badge:     'QLabel',
        label:     'QLabel',
        progress:  'QProgressBar',
        list:      'QListWidget',
        table:     'QTableWidget',
        form:      'QWidget',
        navbar:    'QMenuBar',
        tabgroup:  'QTabWidget',
        divider:   'QFrame',
        container: 'QWidget',
        spinner:   'QProgressBar',
        alert:     'QDialog',
    };
    return map[kind] ?? 'QWidget';
}

function renderTkinterClass(node: WidgetNode, name: string): string {
    const tkWidget = kindToTkWidget(node.kind);
    const className = name.charAt(0).toUpperCase() + name.slice(1);
    const label = node.label ?? node.name ?? name;
    const lines: string[] = [];

    // OW-inspired color palette for Tkinter
    const OW_COLORS = {
        accent:  '#00D4FF',
        bg:      '#0A1628',
        bgCard:  '#0F1F3A',
        text:    '#E0E0E0',
        textDim: '#5588AA',
        border:  '#1E3A5F',
        success: '#00FF88',
        warning: '#FFB800',
        danger:  '#FF4444',
    };

    lines.push(`import tkinter as tk`);
    lines.push(`from tkinter import ttk`);
    lines.push(``);
    lines.push(``);
    lines.push(`class ${className}(tk.Frame):`);
    lines.push(`    """OW-styled ${node.kind} widget — ${label}"""`);
    lines.push(``);
    lines.push(`    def __init__(self, parent, **kwargs):`);
    lines.push(`        super().__init__(`);
    lines.push(`            parent,`);
    lines.push(`            bg="${OW_COLORS.bgCard}",`);
    lines.push(`            **kwargs,`);
    lines.push(`        )`);
    lines.push(`        self._build()`);
    lines.push(``);
    lines.push(`    def _build(self):`);

    if (node.kind === 'button') {
        lines.push(`        self.btn = tk.Button(`);
        lines.push(`            self,`);
        lines.push(`            text="${label}",`);
        lines.push(`            command=self._on_click,`);
        lines.push(`            bg="${OW_COLORS.accent}",`);
        lines.push(`            fg="${OW_COLORS.bg}",`);
        lines.push(`            activebackground="${OW_COLORS.accent}",`);
        lines.push(`            activeforeground="${OW_COLORS.bg}",`);
        lines.push(`            font=("Segoe UI", 10, "bold"),`);
        lines.push(`            relief=tk.FLAT,`);
        lines.push(`            cursor="hand2",`);
        lines.push(`            padx=18,`);
        lines.push(`            pady=8,`);
        lines.push(`            bd=0,`);
        lines.push(`        )`);
        lines.push(`        self.btn.pack(fill=tk.BOTH, expand=True)`);
        if (node.disabled) {
            lines.push(`        self.btn.config(state=tk.DISABLED)`);
        }
    } else if (node.kind === 'input') {
        if (node.placeholder) {
            lines.push(`        # Placeholder text variable`);
            lines.push(`        self._placeholder = "${node.placeholder}"`);
        }
        lines.push(`        self.var = tk.StringVar()`);
        if (node.value) { lines.push(`        self.var.set("${node.value}")`); }
        lines.push(`        self.entry = ttk.Entry(`);
        lines.push(`            self,`);
        lines.push(`            textvariable=self.var,`);
        lines.push(`            font=("Segoe UI", 10),`);
        lines.push(`            style="OW.TEntry",`);
        lines.push(`        )`);
        lines.push(`        self.entry.pack(fill=tk.X, expand=True)`);
        if (node.placeholder) {
            lines.push(`        self.entry.insert(0, self._placeholder)`);
            lines.push(`        self.entry.bind("<FocusIn>", self._on_focus_in)`);
            lines.push(`        self.entry.bind("<FocusOut>", self._on_focus_out)`);
        }
        lines.push(`        self.var.trace_add("write", self._on_change)`);
    } else if (node.kind === 'textarea') {
        lines.push(`        self.text = tk.Text(`);
        lines.push(`            self,`);
        lines.push(`            bg="${OW_COLORS.bgCard}",`);
        lines.push(`            fg="${OW_COLORS.text}",`);
        lines.push(`            insertbackground="${OW_COLORS.accent}",`);
        lines.push(`            font=("Segoe UI", 10),`);
        lines.push(`            relief=tk.FLAT,`);
        lines.push(`            padx=8,`);
        lines.push(`            pady=6,`);
        lines.push(`            wrap=tk.WORD,`);
        lines.push(`            height=6,`);
        lines.push(`        )`);
        lines.push(`        self.text.pack(fill=tk.BOTH, expand=True)`);
        lines.push(`        self.text.bind("<KeyRelease>", self._on_change)`);
    } else if (node.kind === 'toggle' || node.kind === 'checkbox') {
        lines.push(`        self.var = tk.BooleanVar(value=${node.checked ? 'True' : 'False'})`);
        lines.push(`        self.chk = ttk.Checkbutton(`);
        lines.push(`            self,`);
        lines.push(`            text="${label}",`);
        lines.push(`            variable=self.var,`);
        lines.push(`            command=self._on_toggle,`);
        lines.push(`        )`);
        lines.push(`        self.chk.pack(anchor=tk.W)`);
    } else if (node.kind === 'select') {
        lines.push(`        self.var = tk.StringVar()`);
        lines.push(`        self.options = [${(node.options ?? []).map(o => `"${o.label}"`).join(', ') || '"Option 1", "Option 2"'}]`);
        lines.push(`        self.combo = ttk.Combobox(`);
        lines.push(`            self,`);
        lines.push(`            textvariable=self.var,`);
        lines.push(`            values=self.options,`);
        lines.push(`            state="readonly",`);
        lines.push(`            font=("Segoe UI", 10),`);
        lines.push(`        )`);
        lines.push(`        self.combo.current(0)`);
        lines.push(`        self.combo.pack(fill=tk.X, expand=True)`);
        lines.push(`        self.combo.bind("<<ComboboxSelected>>", self._on_change)`);
    } else if (node.kind === 'progress') {
        lines.push(`        self.var = tk.DoubleVar(value=0)`);
        lines.push(`        self.bar = ttk.Progressbar(`);
        lines.push(`            self,`);
        lines.push(`            variable=self.var,`);
        lines.push(`            maximum=100,`);
        lines.push(`            mode="determinate",`);
        lines.push(`            length=300,`);
        lines.push(`        )`);
        lines.push(`        self.bar.pack(fill=tk.X, expand=True, padx=4, pady=4)`);
    } else if (node.kind === 'card' || node.kind === 'panel') {
        lines.push(`        self.frame = ttk.LabelFrame(`);
        lines.push(`            self,`);
        lines.push(`            text="${label}",`);
        lines.push(`            padding=10,`);
        lines.push(`        )`);
        lines.push(`        self.frame.pack(fill=tk.BOTH, expand=True, padx=2, pady=2)`);
    } else {
        lines.push(`        self.widget = ${tkWidget}(`);
        lines.push(`            self,`);
        if (node.label !== undefined) { lines.push(`            text="${label}",`); }
        lines.push(`        )`);
        lines.push(`        self.widget.pack(fill=tk.BOTH, expand=True)`);
    }

    lines.push(``);

    // Event handlers
    if (node.kind === 'button') {
        lines.push(`    def _on_click(self):`);
        lines.push(`        """Handle button click."""`);
        const clickEv = node.events?.find(e => e.name === 'onClick');
        if (clickEv?.handler && !clickEv.handler.startsWith('self.')) {
            lines.push(`        ${clickEv.handler}()`);
        } else {
            lines.push(`        if callable(getattr(self, '_command', None)):`);
            lines.push(`            self._command()`);
        }
        lines.push(``);
    }
    if (node.kind === 'input' || node.kind === 'textarea' || node.kind === 'select') {
        lines.push(`    def _on_change(self, *args):`);
        lines.push(`        """Handle value change."""`);
        lines.push(`        value = self.var.get() if hasattr(self, 'var') else self.text.get('1.0', tk.END).strip()`);
        lines.push(`        if callable(getattr(self, '_on_change_callback', None)):`);
        lines.push(`            self._on_change_callback(value)`);
        lines.push(``);
    }
    if (node.kind === 'toggle' || node.kind === 'checkbox') {
        lines.push(`    def _on_toggle(self):`);
        lines.push(`        """Handle toggle change."""`);
        lines.push(`        checked = self.var.get()`);
        lines.push(`        if callable(getattr(self, '_on_toggle_callback', None)):`);
        lines.push(`            self._on_toggle_callback(checked)`);
        lines.push(``);
    }
    if (node.placeholder && (node.kind === 'input')) {
        lines.push(`    def _on_focus_in(self, event):`);
        lines.push(`        if self.entry.get() == self._placeholder:`);
        lines.push(`            self.entry.delete(0, tk.END)`);
        lines.push(``);
        lines.push(`    def _on_focus_out(self, event):`);
        lines.push(`        if not self.entry.get():`);
        lines.push(`            self.entry.insert(0, self._placeholder)`);
        lines.push(``);
    }

    // Value getters/setters
    if (['input', 'textarea', 'select', 'toggle', 'checkbox'].includes(node.kind)) {
        lines.push(`    def get(self):`);
        lines.push(`        """Get current value."""`);
        if (node.kind === 'textarea') {
            lines.push(`        return self.text.get('1.0', tk.END).strip()`);
        } else {
            lines.push(`        return self.var.get()`);
        }
        lines.push(``);
        lines.push(`    def set(self, value):`);
        lines.push(`        """Set widget value."""`);
        if (node.kind === 'textarea') {
            lines.push(`        self.text.delete('1.0', tk.END)`);
            lines.push(`        self.text.insert('1.0', str(value))`);
        } else {
            lines.push(`        self.var.set(value)`);
        }
    }

    return lines.join('\n');
}

function renderUsageExample(node: WidgetNode, name: string): string {
    const className = name.charAt(0).toUpperCase() + name.slice(1);
    return `
# ── Usage example ──────────────────────────────────────────────────────────────
if __name__ == "__main__":
    root = tk.Tk()
    root.title("${className} Demo")
    root.configure(bg="#0A1628")

    widget = ${className}(root)
    widget.pack(padx=20, pady=20)

    root.mainloop()`;
}

export function generatePython(ir: WidgetIR): string {
    const node = ir.rootWidget;
    const name = ir.name.replace(/[^A-Za-z0-9]/g, '_').replace(/_+/g, '_').replace(/^_|_$/g, '') || 'widget';

    const tkCode = renderTkinterClass(node, name);
    const usageCode = renderUsageExample(node, name);

    return `# ${name} — OW-styled Python GUI Widget (Tkinter)
# Converted from ${ir.sourceLanguage} by Omnisystem Widget Converter
# Confidence: ${ir.confidence}
#
# OW color palette applied — matches omni-dark theme.
# For other themes change the hex color constants in _build().

${tkCode}

${usageCode}
`;
}
