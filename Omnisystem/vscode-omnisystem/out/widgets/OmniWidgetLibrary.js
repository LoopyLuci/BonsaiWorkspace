"use strict";
/// <reference lib="dom" />
/**
 * OmniWidgetLibrary — TypeScript Widget Type System
 * Type definitions, registry, and interfaces for the Modular UI Widget System.
 * TypeScript is used here only for VS Code extension infrastructure.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.OmniWidgetRegistry = exports.OMNI_THEMES = void 0;
exports.OMNI_THEMES = {
    'omni-dark': {
        id: 'omni-dark', label: 'OmniDark', swatch: 'linear-gradient(135deg,#050D1A,#00D4FF)',
        desc: 'Deep space blue',
        tokens: { bg: '#050D1A', bgCard: 'rgba(10,20,42,0.86)', bgRaise: 'rgba(0,20,50,0.52)', glass: 'rgba(8,18,36,0.72)', overlay: 'rgba(0,0,0,0.86)', accent: '#00D4FF', accentDim: 'rgba(0,212,255,0.13)', accentGlow: 'rgba(0,212,255,0.24)', success: '#00FF88', warning: '#FFB800', danger: '#FF4466', purple: '#BF88FF', text: '#E8F4FF', textDim: 'rgba(232,244,255,0.52)', textMuted: 'rgba(232,244,255,0.28)', border: 'rgba(0,212,255,0.18)', borderFocus: 'rgba(0,212,255,0.62)', shadow: '0 3px 16px rgba(0,212,255,0.08)', shadowGlow: '0 0 22px rgba(0,212,255,0.28)' },
    },
    'omni-light': {
        id: 'omni-light', label: 'OmniLight', swatch: 'linear-gradient(135deg,#EEF2FF,#1A6CF0)',
        desc: 'Clean & bright',
        tokens: { bg: '#EEF2FF', bgCard: 'rgba(255,255,255,0.92)', bgRaise: 'rgba(220,230,255,0.62)', glass: 'rgba(240,245,255,0.80)', overlay: 'rgba(0,10,40,0.62)', accent: '#1A6CF0', accentDim: 'rgba(26,108,240,0.12)', accentGlow: 'rgba(26,108,240,0.22)', success: '#0DA84E', warning: '#D07A00', danger: '#D62B4A', purple: '#7C3AED', text: '#0A1230', textDim: 'rgba(10,18,48,0.55)', textMuted: 'rgba(10,18,48,0.32)', border: 'rgba(26,108,240,0.22)', borderFocus: 'rgba(26,108,240,0.72)', shadow: '0 3px 16px rgba(0,0,80,0.08)', shadowGlow: '0 0 22px rgba(26,108,240,0.22)' },
    },
    'omni-neon': {
        id: 'omni-neon', label: 'OmniNeon', swatch: 'linear-gradient(135deg,#000000,#00FF41)',
        desc: 'Terminal green',
        tokens: { bg: '#000000', bgCard: 'rgba(0,18,4,0.90)', bgRaise: 'rgba(0,30,8,0.64)', glass: 'rgba(0,12,2,0.82)', overlay: 'rgba(0,0,0,0.92)', accent: '#00FF41', accentDim: 'rgba(0,255,65,0.12)', accentGlow: 'rgba(0,255,65,0.28)', success: '#00FF41', warning: '#FFFF00', danger: '#FF003C', purple: '#BF00FF', text: '#CCFFDD', textDim: 'rgba(200,255,220,0.52)', textMuted: 'rgba(200,255,220,0.28)', border: 'rgba(0,255,65,0.22)', borderFocus: 'rgba(0,255,65,0.72)', shadow: '0 3px 16px rgba(0,255,65,0.06)', shadowGlow: '0 0 22px rgba(0,255,65,0.32)' },
    },
    'omni-forest': {
        id: 'omni-forest', label: 'OmniForest', swatch: 'linear-gradient(135deg,#050F07,#3CFF7E)',
        desc: 'Deep forest',
        tokens: { bg: '#050F07', bgCard: 'rgba(8,22,10,0.88)', bgRaise: 'rgba(12,32,15,0.58)', glass: 'rgba(6,16,8,0.76)', overlay: 'rgba(0,5,2,0.88)', accent: '#3CFF7E', accentDim: 'rgba(60,255,126,0.12)', accentGlow: 'rgba(60,255,126,0.24)', success: '#7EFF58', warning: '#AAFF00', danger: '#FF4D4D', purple: '#99AAFF', text: '#DDFAE6', textDim: 'rgba(220,250,230,0.52)', textMuted: 'rgba(220,250,230,0.28)', border: 'rgba(60,255,126,0.18)', borderFocus: 'rgba(60,255,126,0.62)', shadow: '0 3px 16px rgba(60,255,126,0.06)', shadowGlow: '0 0 22px rgba(60,255,126,0.26)' },
    },
    'omni-aurora': {
        id: 'omni-aurora', label: 'OmniAurora', swatch: 'linear-gradient(135deg,#0B071E,#C07AFF)',
        desc: 'Violet aurora',
        tokens: { bg: '#0B071E', bgCard: 'rgba(18,12,38,0.88)', bgRaise: 'rgba(28,16,52,0.54)', glass: 'rgba(14,10,28,0.76)', overlay: 'rgba(0,0,10,0.88)', accent: '#C07AFF', accentDim: 'rgba(192,122,255,0.13)', accentGlow: 'rgba(192,122,255,0.26)', success: '#4AFFC8', warning: '#FFD060', danger: '#FF4488', purple: '#C07AFF', text: '#F0EAFF', textDim: 'rgba(240,234,255,0.52)', textMuted: 'rgba(240,234,255,0.28)', border: 'rgba(192,122,255,0.18)', borderFocus: 'rgba(192,122,255,0.62)', shadow: '0 3px 16px rgba(192,122,255,0.08)', shadowGlow: '0 0 22px rgba(192,122,255,0.30)' },
    },
    'omni-sunset': {
        id: 'omni-sunset', label: 'OmniSunset', swatch: 'linear-gradient(135deg,#160700,#FF8C00)',
        desc: 'Warm sunset',
        tokens: { bg: '#160700', bgCard: 'rgba(30,12,0,0.88)', bgRaise: 'rgba(44,16,0,0.54)', glass: 'rgba(22,8,0,0.76)', overlay: 'rgba(10,2,0,0.88)', accent: '#FF8C00', accentDim: 'rgba(255,140,0,0.13)', accentGlow: 'rgba(255,140,0,0.26)', success: '#AAFF44', warning: '#FFD700', danger: '#FF2244', purple: '#FF88CC', text: '#FFF0E0', textDim: 'rgba(255,240,224,0.52)', textMuted: 'rgba(255,240,224,0.28)', border: 'rgba(255,140,0,0.18)', borderFocus: 'rgba(255,140,0,0.62)', shadow: '0 3px 16px rgba(255,140,0,0.08)', shadowGlow: '0 0 22px rgba(255,140,0,0.28)' },
    },
};
class OmniWidgetRegistry {
    static register(descriptor, factory) {
        this._db.set(descriptor.id, descriptor);
        if (factory)
            this._factories.set(descriptor.id, factory);
    }
    static get(id) {
        return this._db.get(id);
    }
    static getFactory(id) {
        return this._factories.get(id);
    }
    static all() {
        return [...this._db.values()];
    }
    static byCategory(cat) {
        return this.all().filter(w => w.cat === cat);
    }
    static categories() {
        return [...new Set(this.all().map(w => w.cat))];
    }
    static search(query) {
        const q = query.toLowerCase();
        return this.all().filter(w => w.label.toLowerCase().includes(q) ||
            w.desc.toLowerCase().includes(q) ||
            w.cat.toLowerCase().includes(q));
    }
    static count() {
        return this._db.size;
    }
}
exports.OmniWidgetRegistry = OmniWidgetRegistry;
OmniWidgetRegistry._db = new Map();
OmniWidgetRegistry._factories = new Map();
//# sourceMappingURL=OmniWidgetLibrary.js.map