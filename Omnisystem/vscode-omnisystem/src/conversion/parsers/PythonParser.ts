// Python GUI Parser — detects widgets from Tkinter, PyQt5/6, PySide, Kivy, wxPython
import {
    WidgetIR, WidgetNode, WidgetKind, WidgetEvent, WidgetProp, WidgetStyle,
    ConversionConfidence, makeId,
} from '../WidgetIR';

interface PyWidgetPattern {
    rx: RegExp;
    kind: WidgetKind;
    framework: string;
}

const TKINTER_PATTERNS: PyWidgetPattern[] = [
    { rx: /(?:tk|ttk)\.Button\s*\(/,        kind: 'button',    framework: 'Tkinter' },
    { rx: /(?:tk|ttk)\.Entry\s*\(/,         kind: 'input',     framework: 'Tkinter' },
    { rx: /(?:tk|ttk)\.Text\s*\(/,          kind: 'textarea',  framework: 'Tkinter' },
    { rx: /(?:tk|ttk)\.Checkbutton\s*\(/,   kind: 'checkbox',  framework: 'Tkinter' },
    { rx: /(?:tk|ttk)\.Radiobutton\s*\(/,   kind: 'radio',     framework: 'Tkinter' },
    { rx: /(?:tk|ttk)\.Scale\s*\(/,         kind: 'slider',    framework: 'Tkinter' },
    { rx: /(?:tk|ttk)\.Combobox\s*\(/,      kind: 'select',    framework: 'Tkinter' },
    { rx: /(?:tk|ttk)\.Listbox\s*\(/,       kind: 'list',      framework: 'Tkinter' },
    { rx: /(?:tk|ttk)\.Treeview\s*\(/,      kind: 'tree',      framework: 'Tkinter' },
    { rx: /(?:tk|ttk)\.Frame\s*\(/,         kind: 'panel',     framework: 'Tkinter' },
    { rx: /(?:tk|ttk)\.LabelFrame\s*\(/,    kind: 'card',      framework: 'Tkinter' },
    { rx: /(?:tk|ttk)\.Notebook\s*\(/,      kind: 'tabgroup',  framework: 'Tkinter' },
    { rx: /(?:tk|ttk)\.Label\s*\(/,         kind: 'label',     framework: 'Tkinter' },
    { rx: /(?:tk|ttk)\.Canvas\s*\(/,        kind: 'chart',     framework: 'Tkinter' },
    { rx: /(?:tk|ttk)\.Progressbar\s*\(/,   kind: 'progress',  framework: 'Tkinter' },
    { rx: /(?:tk|ttk)\.Scrollbar\s*\(/,     kind: 'slider',    framework: 'Tkinter' },
    { rx: /(?:tk|ttk)\.Spinbox\s*\(/,       kind: 'input',     framework: 'Tkinter' },
    { rx: /(?:tk|ttk)\.Menu\s*\(/,          kind: 'navbar',    framework: 'Tkinter' },
    { rx: /(?:tk|ttk)\.Separator\s*\(/,     kind: 'divider',   framework: 'Tkinter' },
    { rx: /(?:tk)\.Toplevel\s*\(/,          kind: 'modal',     framework: 'Tkinter' },
];

const PYQT_PATTERNS: PyWidgetPattern[] = [
    { rx: /QPushButton\s*\(/,           kind: 'button',    framework: 'PyQt' },
    { rx: /QLineEdit\s*\(/,             kind: 'input',     framework: 'PyQt' },
    { rx: /QTextEdit\s*\(/,             kind: 'textarea',  framework: 'PyQt' },
    { rx: /QPlainTextEdit\s*\(/,        kind: 'textarea',  framework: 'PyQt' },
    { rx: /QCheckBox\s*\(/,             kind: 'checkbox',  framework: 'PyQt' },
    { rx: /QRadioButton\s*\(/,          kind: 'radio',     framework: 'PyQt' },
    { rx: /QSlider\s*\(/,               kind: 'slider',    framework: 'PyQt' },
    { rx: /QComboBox\s*\(/,             kind: 'select',    framework: 'PyQt' },
    { rx: /QListWidget\s*\(/,           kind: 'list',      framework: 'PyQt' },
    { rx: /QListView\s*\(/,             kind: 'list',      framework: 'PyQt' },
    { rx: /QTreeView\s*\(/,             kind: 'tree',      framework: 'PyQt' },
    { rx: /QTableWidget\s*\(/,          kind: 'table',     framework: 'PyQt' },
    { rx: /QTableView\s*\(/,            kind: 'table',     framework: 'PyQt' },
    { rx: /QGroupBox\s*\(/,             kind: 'card',      framework: 'PyQt' },
    { rx: /QFrame\s*\(/,                kind: 'panel',     framework: 'PyQt' },
    { rx: /QWidget\s*\(/,               kind: 'container', framework: 'PyQt' },
    { rx: /QTabWidget\s*\(/,            kind: 'tabgroup',  framework: 'PyQt' },
    { rx: /QLabel\s*\(/,                kind: 'label',     framework: 'PyQt' },
    { rx: /QProgressBar\s*\(/,          kind: 'progress',  framework: 'PyQt' },
    { rx: /QDialog\s*\(/,               kind: 'modal',     framework: 'PyQt' },
    { rx: /QScrollArea\s*\(/,           kind: 'container', framework: 'PyQt' },
    { rx: /QMenuBar\s*\(/,              kind: 'navbar',    framework: 'PyQt' },
    { rx: /QStatusBar\s*\(/,            kind: 'panel',     framework: 'PyQt' },
    { rx: /QToolBar\s*\(/,              kind: 'navbar',    framework: 'PyQt' },
    { rx: /QSplitter\s*\(/,             kind: 'panel',     framework: 'PyQt' },
    { rx: /QSpinBox\s*\(/,              kind: 'input',     framework: 'PyQt' },
    { rx: /QDoubleSpinBox\s*\(/,        kind: 'input',     framework: 'PyQt' },
    { rx: /QDateEdit\s*\(/,             kind: 'datepicker',framework: 'PyQt' },
    { rx: /QColorDialog\s*\(/,          kind: 'colorpicker',framework: 'PyQt' },
];

const KIVY_PATTERNS: PyWidgetPattern[] = [
    { rx: /Button\s*:/,              kind: 'button',    framework: 'Kivy' },
    { rx: /TextInput\s*:/,           kind: 'input',     framework: 'Kivy' },
    { rx: /CheckBox\s*:/,            kind: 'checkbox',  framework: 'Kivy' },
    { rx: /Slider\s*:/,              kind: 'slider',    framework: 'Kivy' },
    { rx: /DropDown\s*:|Spinner\s*:/, kind: 'select',   framework: 'Kivy' },
    { rx: /BoxLayout\s*:/,           kind: 'panel',     framework: 'Kivy' },
    { rx: /GridLayout\s*:/,          kind: 'grid',      framework: 'Kivy' },
    { rx: /TabbedPanel\s*:/,         kind: 'tabgroup',  framework: 'Kivy' },
    { rx: /Label\s*:/,               kind: 'label',     framework: 'Kivy' },
    { rx: /ProgressBar\s*:/,         kind: 'progress',  framework: 'Kivy' },
    { rx: /Popup\s*:/,               kind: 'modal',     framework: 'Kivy' },
    { rx: /Image\s*:/,               kind: 'image',     framework: 'Kivy' },
    { rx: /ScrollView\s*:/,          kind: 'container', framework: 'Kivy' },
    { rx: /Switch\s*:/,              kind: 'toggle',    framework: 'Kivy' },
    { rx: /ToggleButton\s*:/,        kind: 'toggle',    framework: 'Kivy' },
];

function extractTkProps(src: string, widgetCall: string): { text?: string; command?: string; variable?: string; width?: string; height?: string; bg?: string } {
    // Find the constructor call for this widget
    const callRx = new RegExp(`${widgetCall}\\(([^)]{0,500})\\)`, 's');
    const m = src.match(callRx);
    if (!m) { return {}; }
    const args = m[1];

    function extractArg(name: string): string | undefined {
        const r = new RegExp(`\\b${name}\\s*=\\s*['"]([^'"]+)['"]|\\b${name}\\s*=\\s*([\\w.]+)`, 'i');
        const a = args.match(r);
        return a ? (a[1] ?? a[2]) : undefined;
    }

    return {
        text: extractArg('text'),
        command: extractArg('command'),
        variable: extractArg('variable') ?? extractArg('textvariable'),
        width: extractArg('width'),
        height: extractArg('height'),
        bg: extractArg('bg') ?? extractArg('background'),
    };
}

function extractPyQtSignals(src: string): WidgetEvent[] {
    const events: WidgetEvent[] = [];
    // clicked.connect, textChanged.connect, valueChanged.connect, etc.
    const signalRx = /\.(\w+)\.connect\s*\(\s*([^)]+)\s*\)/g;
    let m: RegExpExecArray | null;
    while ((m = signalRx.exec(src)) !== null) {
        const signalName = m[1];
        const handler = m[2].trim();
        const eventName = signalName === 'clicked' ? 'onClick' :
                          signalName === 'textChanged' ? 'onChange' :
                          signalName === 'valueChanged' ? 'onChange' :
                          signalName === 'returnPressed' ? 'onSubmit' :
                          signalName === 'toggled' ? 'onChange' :
                          signalName === 'activated' ? 'onChange' :
                          'on' + signalName.charAt(0).toUpperCase() + signalName.slice(1);
        events.push({ name: eventName, handler, params: [] });
    }
    return events;
}

function extractClassName(src: string): string {
    // class MyWidget(QWidget): or class MyApp(tk.Tk):
    const rx = /class\s+([A-Z][A-Za-z0-9]*)\s*(?:\([^)]+\))?\s*:/;
    return src.match(rx)?.[1] ?? 'Widget';
}

export function parsePython(source: string): WidgetIR {
    const src = source.trim();
    const notes: string[] = [];

    let detectedKind: WidgetKind = 'unknown';
    let framework = 'Unknown';
    let label: string | undefined;
    let events: WidgetEvent[] = [];
    let confidence: ConversionConfidence = 'low';

    // Detect framework and widget
    const allPatterns = [...TKINTER_PATTERNS, ...PYQT_PATTERNS, ...KIVY_PATTERNS];
    for (const pat of allPatterns) {
        if (pat.rx.test(src)) {
            detectedKind = pat.kind;
            framework = pat.framework;
            confidence = 'high';
            break;
        }
    }

    if (framework === 'Tkinter') {
        // Find which pattern matched
        for (const pat of TKINTER_PATTERNS) {
            if (pat.rx.test(src)) {
                const callMatch = src.match(pat.rx);
                if (callMatch) {
                    const widgetCall = callMatch[0].replace(/\s*\($/, '');
                    const tkProps = extractTkProps(src, widgetCall.replace(/.*\./, ''));
                    label = tkProps.text;
                    if (tkProps.command) {
                        events.push({ name: 'onClick', handler: tkProps.command, params: [] });
                    }
                }
                break;
            }
        }
        notes.push(`Tkinter widget detected: ${detectedKind}`);
    } else if (framework === 'PyQt') {
        events = extractPyQtSignals(src);
        // Extract text argument from QPushButton("label")
        const textArgRx = /(?:QPushButton|QLabel|QCheckBox|QRadioButton|QGroupBox|QAction)\s*\(\s*(?:self,\s*)?["']([^"']+)["']/;
        label = src.match(textArgRx)?.[1];
        notes.push(`PyQt/PySide widget detected: ${detectedKind}`);
    } else if (framework === 'Kivy') {
        const textRx = /text:\s*['"]([^'"]+)['"]/;
        label = src.match(textRx)?.[1];
        notes.push(`Kivy widget detected: ${detectedKind}`);
    }

    if (detectedKind === 'unknown') {
        notes.push('No recognized Python GUI widget pattern found');
    }

    if (events.length > 0) {
        notes.push(`${events.length} event handler(s) extracted`);
    }

    const className = extractClassName(src);

    const node: WidgetNode = {
        id: makeId(className),
        kind: detectedKind,
        name: className,
        label,
        events,
        meta: { framework, sourceLanguage: 'python' },
    };

    return {
        name: className,
        rootWidget: node,
        sourceLanguage: 'python',
        confidence,
        notes,
    };
}
