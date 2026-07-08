"use strict";
// Universal Widget Intermediate Representation
// Shared types used by all parsers and generators
Object.defineProperty(exports, "__esModule", { value: true });
exports.LANGUAGE_LABELS = exports.LANGUAGE_EXTENSIONS = void 0;
exports.makeId = makeId;
function makeId(seed) {
    return seed.toLowerCase().replace(/[^a-z0-9]/g, '_').replace(/_+/g, '_').replace(/^_|_$/g, '') || 'widget';
}
exports.LANGUAGE_EXTENSIONS = {
    vera: '.vera',
    nexus: '.nexus',
    titan: '.titan',
    javascript: '.js',
    typescript: '.ts',
    css: '.css',
    tauri: '.html',
    python: '.py',
};
exports.LANGUAGE_LABELS = {
    javascript: 'JavaScript',
    typescript: 'TypeScript',
    css: 'CSS',
    tauri: 'Tauri (HTML+JS)',
    python: 'Python GUI (Tkinter/PyQt)',
    vera: 'Vera (OW Component)',
    nexus: 'Nexus (OW Layout)',
    titan: 'Titan (OW Runtime)',
};
//# sourceMappingURL=WidgetIR.js.map