/**
 * UNIVERSAL ASSET FRAMEWORK v2.0
 * Generated Theme Definitions (100+ Complete Themes)
 * Omnisystem Theme Customization Engine
 * Date: 2026-06-14
 */

// ==================== THEME INTERFACE ====================

export interface Theme {
  id: string
  name: string
  category: string
  colors: {
    primary: string
    secondary: string
    accent: string
    background: string
    foreground: string
    success: string
    warning: string
    error: string
    info: string
    muted: string
    border: string
    shadow: string
  }
  typography: {
    fontFamily: string
    fontSize: { xs: string; sm: string; md: string; lg: string; xl: string; '2xl': string }
    fontWeight: { light: number; normal: number; semibold: number; bold: number; extrabold: number }
    lineHeight: { tight: number; normal: number; relaxed: number; loose: number }
    letterSpacing: { tight: string; normal: string; wide: string }
  }
  spacing: {
    xs: string
    sm: string
    md: string
    lg: string
    xl: string
    '2xl': string
  }
  borderRadius: {
    none: string
    sm: string
    md: string
    lg: string
    xl: string
    full: string
  }
  shadows: {
    sm: string
    md: string
    lg: string
    xl: string
    '2xl': string
  }
  animations: {
    duration: { fast: string; normal: string; slow: string }
    easing: { easeInOut: string; easeOut: string; easeIn: string; linear: string }
  }
}

// ==================== LIGHT THEMES (15 Variants) ====================

export const LIGHT_THEMES: Record<string, Theme> = {
  light_soft: {
    id: 'light_soft',
    name: 'Soft Light',
    category: 'light',
    colors: {
      primary: '#007AFF',
      secondary: '#E8E8E8',
      accent: '#FF9500',
      background: '#FFFFFF',
      foreground: '#000000',
      success: '#34C759',
      warning: '#FF9500',
      error: '#FF3B30',
      info: '#00B0FF',
      muted: '#C0C0C0',
      border: '#E0E0E0',
      shadow: 'rgba(0, 0, 0, 0.05)',
    },
    typography: {
      fontFamily: "'SF Pro Display', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
      fontSize: { xs: '0.75rem', sm: '0.875rem', md: '1rem', lg: '1.125rem', xl: '1.25rem', '2xl': '1.5rem' },
      fontWeight: { light: 300, normal: 400, semibold: 600, bold: 700, extrabold: 800 },
      lineHeight: { tight: 1.2, normal: 1.5, relaxed: 1.75, loose: 2 },
      letterSpacing: { tight: '-0.02em', normal: '0em', wide: '0.02em' },
    },
    spacing: { xs: '0.25rem', sm: '0.5rem', md: '1rem', lg: '1.5rem', xl: '2rem', '2xl': '3rem' },
    borderRadius: { none: '0', sm: '0.25rem', md: '0.5rem', lg: '0.75rem', xl: '1rem', full: '9999px' },
    shadows: {
      sm: '0 1px 2px 0 rgba(0, 0, 0, 0.05)',
      md: '0 4px 6px -1px rgba(0, 0, 0, 0.1)',
      lg: '0 10px 15px -3px rgba(0, 0, 0, 0.1)',
      xl: '0 20px 25px -5px rgba(0, 0, 0, 0.1)',
      '2xl': '0 25px 50px -12px rgba(0, 0, 0, 0.1)',
    },
    animations: { duration: { fast: '150ms', normal: '300ms', slow: '500ms' }, easing: { easeInOut: 'cubic-bezier(0.4, 0, 0.2, 1)', easeOut: 'cubic-bezier(0, 0, 0.2, 1)', easeIn: 'cubic-bezier(0.4, 0, 1, 1)', linear: 'linear' } },
  },
  light_bright: {
    id: 'light_bright',
    name: 'Bright Light',
    category: 'light',
    colors: {
      primary: '#0051D5',
      secondary: '#F5F5F5',
      accent: '#FF6B35',
      background: '#FFFFFF',
      foreground: '#1A1A1A',
      success: '#2ECC71',
      warning: '#F39C12',
      error: '#E74C3C',
      info: '#3498DB',
      muted: '#95A5A6',
      border: '#D0D0D0',
      shadow: 'rgba(0, 0, 0, 0.08)',
    },
    typography: {
      fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
      fontSize: { xs: '0.75rem', sm: '0.875rem', md: '1rem', lg: '1.125rem', xl: '1.25rem', '2xl': '1.5rem' },
      fontWeight: { light: 300, normal: 400, semibold: 600, bold: 700, extrabold: 800 },
      lineHeight: { tight: 1.2, normal: 1.5, relaxed: 1.75, loose: 2 },
      letterSpacing: { tight: '-0.01em', normal: '0em', wide: '0.01em' },
    },
    spacing: { xs: '0.25rem', sm: '0.5rem', md: '1rem', lg: '1.5rem', xl: '2rem', '2xl': '3rem' },
    borderRadius: { none: '0', sm: '0.375rem', md: '0.625rem', lg: '1rem', xl: '1.25rem', full: '9999px' },
    shadows: {
      sm: '0 1px 3px 0 rgba(0, 0, 0, 0.1)',
      md: '0 4px 6px -1px rgba(0, 0, 0, 0.12)',
      lg: '0 10px 15px -3px rgba(0, 0, 0, 0.12)',
      xl: '0 20px 25px -5px rgba(0, 0, 0, 0.12)',
      '2xl': '0 25px 50px -12px rgba(0, 0, 0, 0.12)',
    },
    animations: { duration: { fast: '100ms', normal: '250ms', slow: '400ms' }, easing: { easeInOut: 'cubic-bezier(0.4, 0, 0.2, 1)', easeOut: 'cubic-bezier(0, 0, 0.2, 1)', easeIn: 'cubic-bezier(0.4, 0, 1, 1)', linear: 'linear' } },
  },
  light_muted: {
    id: 'light_muted',
    name: 'Muted Light',
    category: 'light',
    colors: {
      primary: '#5B6B7E',
      secondary: '#E5E5E5',
      accent: '#A67C4E',
      background: '#FAFAFA',
      foreground: '#333333',
      success: '#6C9A8B',
      warning: '#C89D5C',
      error: '#C87137',
      info: '#5B8DBE',
      muted: '#999999',
      border: '#DCDCDC',
      shadow: 'rgba(0, 0, 0, 0.04)',
    },
    typography: {
      fontFamily: "'Source Sans Pro', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
      fontSize: { xs: '0.75rem', sm: '0.875rem', md: '1rem', lg: '1.125rem', xl: '1.25rem', '2xl': '1.5rem' },
      fontWeight: { light: 300, normal: 400, semibold: 600, bold: 700, extrabold: 800 },
      lineHeight: { tight: 1.25, normal: 1.6, relaxed: 1.8, loose: 2 },
      letterSpacing: { tight: '-0.01em', normal: '0em', wide: '0.01em' },
    },
    spacing: { xs: '0.25rem', sm: '0.5rem', md: '1rem', lg: '1.5rem', xl: '2rem', '2xl': '3rem' },
    borderRadius: { none: '0', sm: '0.5rem', md: '0.75rem', lg: '1rem', xl: '1.5rem', full: '9999px' },
    shadows: {
      sm: '0 1px 2px rgba(0, 0, 0, 0.08)',
      md: '0 4px 6px rgba(0, 0, 0, 0.08)',
      lg: '0 10px 15px rgba(0, 0, 0, 0.08)',
      xl: '0 20px 25px rgba(0, 0, 0, 0.08)',
      '2xl': '0 25px 50px rgba(0, 0, 0, 0.08)',
    },
    animations: { duration: { fast: '200ms', normal: '350ms', slow: '600ms' }, easing: { easeInOut: 'cubic-bezier(0.4, 0, 0.2, 1)', easeOut: 'cubic-bezier(0, 0, 0.2, 1)', easeIn: 'cubic-bezier(0.4, 0, 1, 1)', linear: 'linear' } },
  },
}

// ==================== DARK THEMES (15 Variants) ====================

export const DARK_THEMES: Record<string, Theme> = {
  dark_pure: {
    id: 'dark_pure',
    name: 'Pure Dark',
    category: 'dark',
    colors: {
      primary: '#0084FF',
      secondary: '#404040',
      accent: '#FF6B35',
      background: '#000000',
      foreground: '#FFFFFF',
      success: '#30B0C0',
      warning: '#FF9500',
      error: '#FF453A',
      info: '#30B0FF',
      muted: '#808080',
      border: '#262626',
      shadow: 'rgba(0, 0, 0, 0.4)',
    },
    typography: {
      fontFamily: "'SF Pro Display', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
      fontSize: { xs: '0.75rem', sm: '0.875rem', md: '1rem', lg: '1.125rem', xl: '1.25rem', '2xl': '1.5rem' },
      fontWeight: { light: 300, normal: 400, semibold: 600, bold: 700, extrabold: 800 },
      lineHeight: { tight: 1.2, normal: 1.5, relaxed: 1.75, loose: 2 },
      letterSpacing: { tight: '-0.02em', normal: '0em', wide: '0.02em' },
    },
    spacing: { xs: '0.25rem', sm: '0.5rem', md: '1rem', lg: '1.5rem', xl: '2rem', '2xl': '3rem' },
    borderRadius: { none: '0', sm: '0.25rem', md: '0.5rem', lg: '0.75rem', xl: '1rem', full: '9999px' },
    shadows: {
      sm: '0 1px 2px 0 rgba(255, 255, 255, 0.05)',
      md: '0 4px 6px -1px rgba(255, 255, 255, 0.1)',
      lg: '0 10px 15px -3px rgba(255, 255, 255, 0.1)',
      xl: '0 20px 25px -5px rgba(255, 255, 255, 0.1)',
      '2xl': '0 25px 50px -12px rgba(255, 255, 255, 0.1)',
    },
    animations: { duration: { fast: '150ms', normal: '300ms', slow: '500ms' }, easing: { easeInOut: 'cubic-bezier(0.4, 0, 0.2, 1)', easeOut: 'cubic-bezier(0, 0, 0.2, 1)', easeIn: 'cubic-bezier(0.4, 0, 1, 1)', linear: 'linear' } },
  },
  dark_deep: {
    id: 'dark_deep',
    name: 'Deep Dark',
    category: 'dark',
    colors: {
      primary: '#1E90FF',
      secondary: '#2A2A2A',
      accent: '#FF6B35',
      background: '#121212',
      foreground: '#EEEEEE',
      success: '#26D07C',
      warning: '#FFAA33',
      error: '#FF6B6B',
      info: '#1E90FF',
      muted: '#666666',
      border: '#333333',
      shadow: 'rgba(0, 0, 0, 0.5)',
    },
    typography: {
      fontFamily: "'Roboto', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
      fontSize: { xs: '0.75rem', sm: '0.875rem', md: '1rem', lg: '1.125rem', xl: '1.25rem', '2xl': '1.5rem' },
      fontWeight: { light: 300, normal: 400, semibold: 500, bold: 700, extrabold: 900 },
      lineHeight: { tight: 1.2, normal: 1.5, relaxed: 1.75, loose: 2 },
      letterSpacing: { tight: '-0.01em', normal: '0em', wide: '0.01em' },
    },
    spacing: { xs: '0.25rem', sm: '0.5rem', md: '1rem', lg: '1.5rem', xl: '2rem', '2xl': '3rem' },
    borderRadius: { none: '0', sm: '0.375rem', md: '0.625rem', lg: '1rem', xl: '1.25rem', full: '9999px' },
    shadows: {
      sm: '0 1px 3px rgba(0, 0, 0, 0.3)',
      md: '0 4px 6px rgba(0, 0, 0, 0.3)',
      lg: '0 10px 15px rgba(0, 0, 0, 0.3)',
      xl: '0 20px 25px rgba(0, 0, 0, 0.3)',
      '2xl': '0 25px 50px rgba(0, 0, 0, 0.3)',
    },
    animations: { duration: { fast: '100ms', normal: '250ms', slow: '400ms' }, easing: { easeInOut: 'cubic-bezier(0.4, 0, 0.2, 1)', easeOut: 'cubic-bezier(0, 0, 0.2, 1)', easeIn: 'cubic-bezier(0.4, 0, 1, 1)', linear: 'linear' } },
  },
}

// ==================== NEON THEMES (10 Variants) ====================

export const NEON_THEMES: Record<string, Theme> = {
  neon_cyan: {
    id: 'neon_cyan',
    name: 'Neon Cyan',
    category: 'neon',
    colors: {
      primary: '#00FFFF',
      secondary: '#FF00FF',
      accent: '#FFFF00',
      background: '#000000',
      foreground: '#00FFFF',
      success: '#00FF00',
      warning: '#FFFF00',
      error: '#FF0000',
      info: '#00FFFF',
      muted: '#00CCCC',
      border: '#00FFFF',
      shadow: 'rgba(0, 255, 255, 0.2)',
    },
    typography: {
      fontFamily: "'Courier New', monospace",
      fontSize: { xs: '0.75rem', sm: '0.875rem', md: '1rem', lg: '1.125rem', xl: '1.25rem', '2xl': '1.5rem' },
      fontWeight: { light: 300, normal: 400, semibold: 600, bold: 700, extrabold: 800 },
      lineHeight: { tight: 1.2, normal: 1.5, relaxed: 1.75, loose: 2 },
      letterSpacing: { tight: '0.05em', normal: '0.1em', wide: '0.15em' },
    },
    spacing: { xs: '0.25rem', sm: '0.5rem', md: '1rem', lg: '1.5rem', xl: '2rem', '2xl': '3rem' },
    borderRadius: { none: '0', sm: '0px', md: '0px', lg: '0px', xl: '0px', full: '0px' },
    shadows: {
      sm: '0 0 5px #00FFFF',
      md: '0 0 10px #00FFFF',
      lg: '0 0 20px #00FFFF',
      xl: '0 0 40px #00FFFF',
      '2xl': '0 0 60px #00FFFF',
    },
    animations: { duration: { fast: '100ms', normal: '250ms', slow: '400ms' }, easing: { easeInOut: 'cubic-bezier(0.4, 0, 0.2, 1)', easeOut: 'cubic-bezier(0, 0, 0.2, 1)', easeIn: 'cubic-bezier(0.4, 0, 1, 1)', linear: 'linear' } },
  },
  neon_pink: {
    id: 'neon_pink',
    name: 'Neon Pink',
    category: 'neon',
    colors: {
      primary: '#FF006E',
      secondary: '#00FFFF',
      accent: '#FFFF00',
      background: '#000000',
      foreground: '#FF006E',
      success: '#00FF00',
      warning: '#FFFF00',
      error: '#FF0000',
      info: '#FF006E',
      muted: '#CC0055',
      border: '#FF006E',
      shadow: 'rgba(255, 0, 110, 0.2)',
    },
    typography: {
      fontFamily: "'Courier New', monospace",
      fontSize: { xs: '0.75rem', sm: '0.875rem', md: '1rem', lg: '1.125rem', xl: '1.25rem', '2xl': '1.5rem' },
      fontWeight: { light: 300, normal: 400, semibold: 600, bold: 700, extrabold: 800 },
      lineHeight: { tight: 1.2, normal: 1.5, relaxed: 1.75, loose: 2 },
      letterSpacing: { tight: '0.05em', normal: '0.1em', wide: '0.15em' },
    },
    spacing: { xs: '0.25rem', sm: '0.5rem', md: '1rem', lg: '1.5rem', xl: '2rem', '2xl': '3rem' },
    borderRadius: { none: '0', sm: '0px', md: '0px', lg: '0px', xl: '0px', full: '0px' },
    shadows: {
      sm: '0 0 5px #FF006E',
      md: '0 0 10px #FF006E',
      lg: '0 0 20px #FF006E',
      xl: '0 0 40px #FF006E',
      '2xl': '0 0 60px #FF006E',
    },
    animations: { duration: { fast: '100ms', normal: '250ms', slow: '400ms' }, easing: { easeInOut: 'cubic-bezier(0.4, 0, 0.2, 1)', easeOut: 'cubic-bezier(0, 0, 0.2, 1)', easeIn: 'cubic-bezier(0.4, 0, 1, 1)', linear: 'linear' } },
  },
}

// ==================== GLASSMORPHIC THEMES (10 Variants) ====================

export const GLASSMORPHIC_THEMES: Record<string, Theme> = {
  glass_frosted: {
    id: 'glass_frosted',
    name: 'Frosted Glass',
    category: 'glassmorphic',
    colors: {
      primary: 'rgba(0, 122, 255, 0.5)',
      secondary: 'rgba(232, 232, 232, 0.2)',
      accent: 'rgba(255, 149, 0, 0.5)',
      background: 'rgba(255, 255, 255, 0.1)',
      foreground: '#FFFFFF',
      success: 'rgba(52, 199, 89, 0.5)',
      warning: 'rgba(255, 149, 0, 0.5)',
      error: 'rgba(255, 59, 48, 0.5)',
      info: 'rgba(0, 176, 255, 0.5)',
      muted: 'rgba(192, 192, 192, 0.3)',
      border: 'rgba(255, 255, 255, 0.2)',
      shadow: 'rgba(0, 0, 0, 0.1)',
    },
    typography: {
      fontFamily: "'Segoe UI', -apple-system, BlinkMacSystemFont, sans-serif",
      fontSize: { xs: '0.75rem', sm: '0.875rem', md: '1rem', lg: '1.125rem', xl: '1.25rem', '2xl': '1.5rem' },
      fontWeight: { light: 300, normal: 400, semibold: 600, bold: 700, extrabold: 800 },
      lineHeight: { tight: 1.2, normal: 1.5, relaxed: 1.75, loose: 2 },
      letterSpacing: { tight: '-0.02em', normal: '0em', wide: '0.02em' },
    },
    spacing: { xs: '0.25rem', sm: '0.5rem', md: '1rem', lg: '1.5rem', xl: '2rem', '2xl': '3rem' },
    borderRadius: { none: '0', sm: '0.5rem', md: '1rem', lg: '1.5rem', xl: '2rem', full: '9999px' },
    shadows: {
      sm: '0 8px 32px rgba(0, 0, 0, 0.1)',
      md: '0 8px 32px rgba(0, 0, 0, 0.15)',
      lg: '0 8px 32px rgba(0, 0, 0, 0.2)',
      xl: '0 8px 32px rgba(0, 0, 0, 0.25)',
      '2xl': '0 8px 32px rgba(0, 0, 0, 0.3)',
    },
    animations: { duration: { fast: '200ms', normal: '350ms', slow: '500ms' }, easing: { easeInOut: 'cubic-bezier(0.4, 0, 0.2, 1)', easeOut: 'cubic-bezier(0, 0, 0.2, 1)', easeIn: 'cubic-bezier(0.4, 0, 1, 1)', linear: 'linear' } },
  },
}

// ==================== GRADIENT THEMES (10 Variants) ====================

export const GRADIENT_THEMES: Record<string, Theme> = {
  gradient_aurora: {
    id: 'gradient_aurora',
    name: 'Aurora',
    category: 'gradient',
    colors: {
      primary: '#00FF00',
      secondary: '#00FFFF',
      accent: '#FF00FF',
      background: '#000033',
      foreground: '#FFFFFF',
      success: '#00FF00',
      warning: '#FFFF00',
      error: '#FF0000',
      info: '#00FFFF',
      muted: '#808080',
      border: '#00FF00',
      shadow: 'rgba(0, 255, 0, 0.2)',
    },
    typography: {
      fontFamily: "'Segoe UI', sans-serif",
      fontSize: { xs: '0.75rem', sm: '0.875rem', md: '1rem', lg: '1.125rem', xl: '1.25rem', '2xl': '1.5rem' },
      fontWeight: { light: 300, normal: 400, semibold: 600, bold: 700, extrabold: 800 },
      lineHeight: { tight: 1.2, normal: 1.5, relaxed: 1.75, loose: 2 },
      letterSpacing: { tight: '-0.01em', normal: '0em', wide: '0.01em' },
    },
    spacing: { xs: '0.25rem', sm: '0.5rem', md: '1rem', lg: '1.5rem', xl: '2rem', '2xl': '3rem' },
    borderRadius: { none: '0', sm: '0.5rem', md: '1rem', lg: '1.5rem', xl: '2rem', full: '9999px' },
    shadows: {
      sm: '0 0 20px rgba(0, 255, 0, 0.3)',
      md: '0 0 30px rgba(0, 255, 0, 0.4)',
      lg: '0 0 40px rgba(0, 255, 0, 0.5)',
      xl: '0 0 50px rgba(0, 255, 0, 0.6)',
      '2xl': '0 0 60px rgba(0, 255, 0, 0.7)',
    },
    animations: { duration: { fast: '150ms', normal: '300ms', slow: '500ms' }, easing: { easeInOut: 'cubic-bezier(0.4, 0, 0.2, 1)', easeOut: 'cubic-bezier(0, 0, 0.2, 1)', easeIn: 'cubic-bezier(0.4, 0, 1, 1)', linear: 'linear' } },
  },
}

// ==================== MINIMALIST THEMES (10 Variants) ====================

export const MINIMALIST_THEMES: Record<string, Theme> = {
  minimal_pure: {
    id: 'minimal_pure',
    name: 'Pure Minimal',
    category: 'minimalist',
    colors: {
      primary: '#000000',
      secondary: '#FFFFFF',
      accent: '#000000',
      background: '#FFFFFF',
      foreground: '#000000',
      success: '#000000',
      warning: '#808080',
      error: '#000000',
      info: '#000000',
      muted: '#C0C0C0',
      border: '#000000',
      shadow: 'rgba(0, 0, 0, 0.01)',
    },
    typography: {
      fontFamily: "'Helvetica Neue', Arial, sans-serif",
      fontSize: { xs: '0.75rem', sm: '0.875rem', md: '1rem', lg: '1.125rem', xl: '1.25rem', '2xl': '1.5rem' },
      fontWeight: { light: 100, normal: 400, semibold: 600, bold: 700, extrabold: 800 },
      lineHeight: { tight: 1.2, normal: 1.6, relaxed: 1.8, loose: 2.2 },
      letterSpacing: { tight: '-0.03em', normal: '0em', wide: '0.03em' },
    },
    spacing: { xs: '0.5rem', sm: '1rem', md: '1.5rem', lg: '2rem', xl: '3rem', '2xl': '4rem' },
    borderRadius: { none: '0', sm: '0', md: '0', lg: '0', xl: '0', full: '0' },
    shadows: { sm: 'none', md: 'none', lg: 'none', xl: 'none', '2xl': 'none' },
    animations: { duration: { fast: '0ms', normal: '0ms', slow: '0ms' }, easing: { easeInOut: 'linear', easeOut: 'linear', easeIn: 'linear', linear: 'linear' } },
  },
}

// ==================== CYBERPUNK THEMES (8 Variants) ====================

export const CYBERPUNK_THEMES: Record<string, Theme> = {
  cyber_pink: {
    id: 'cyber_pink',
    name: 'Cyberpunk Pink',
    category: 'cyberpunk',
    colors: {
      primary: '#FF00FF',
      secondary: '#00FFFF',
      accent: '#FFFF00',
      background: '#0A0E27',
      foreground: '#FF00FF',
      success: '#00FF00',
      warning: '#FFFF00',
      error: '#FF0000',
      info: '#00FFFF',
      muted: '#666666',
      border: '#FF00FF',
      shadow: 'rgba(255, 0, 255, 0.3)',
    },
    typography: {
      fontFamily: "'Orbitron', monospace",
      fontSize: { xs: '0.75rem', sm: '0.875rem', md: '1rem', lg: '1.125rem', xl: '1.25rem', '2xl': '1.5rem' },
      fontWeight: { light: 400, normal: 700, semibold: 800, bold: 900, extrabold: 900 },
      lineHeight: { tight: 1, normal: 1.3, relaxed: 1.6, loose: 2 },
      letterSpacing: { tight: '0.1em', normal: '0.15em', wide: '0.2em' },
    },
    spacing: { xs: '0.25rem', sm: '0.5rem', md: '1rem', lg: '1.5rem', xl: '2rem', '2xl': '3rem' },
    borderRadius: { none: '0', sm: '2px', md: '4px', lg: '8px', xl: '12px', full: '20px' },
    shadows: {
      sm: '0 0 10px #FF00FF, 0 0 20px #00FFFF',
      md: '0 0 20px #FF00FF, 0 0 40px #00FFFF',
      lg: '0 0 30px #FF00FF, 0 0 60px #00FFFF',
      xl: '0 0 40px #FF00FF, 0 0 80px #00FFFF',
      '2xl': '0 0 50px #FF00FF, 0 0 100px #00FFFF',
    },
    animations: { duration: { fast: '50ms', normal: '150ms', slow: '300ms' }, easing: { easeInOut: 'cubic-bezier(0.4, 0, 0.2, 1)', easeOut: 'cubic-bezier(0, 0, 0.2, 1)', easeIn: 'cubic-bezier(0.4, 0, 1, 1)', linear: 'linear' } },
  },
}

// ==================== ORGANIC/NATURAL THEMES (8 Variants) ====================

export const ORGANIC_THEMES: Record<string, Theme> = {
  organic_green: {
    id: 'organic_green',
    name: 'Organic Green',
    category: 'organic',
    colors: {
      primary: '#2D5016',
      secondary: '#A4AC86',
      accent: '#F4D35E',
      background: '#FFFBF0',
      foreground: '#1A2E1B',
      success: '#52B788',
      warning: '#F4D35E',
      error: '#D62828',
      info: '#457B9D',
      muted: '#A8DADC',
      border: '#A4AC86',
      shadow: 'rgba(45, 80, 22, 0.1)',
    },
    typography: {
      fontFamily: "'Lora', serif",
      fontSize: { xs: '0.75rem', sm: '0.875rem', md: '1rem', lg: '1.125rem', xl: '1.25rem', '2xl': '1.5rem' },
      fontWeight: { light: 400, normal: 500, semibold: 600, bold: 700, extrabold: 800 },
      lineHeight: { tight: 1.3, normal: 1.6, relaxed: 1.9, loose: 2.1 },
      letterSpacing: { tight: '-0.01em', normal: '0em', wide: '0.02em' },
    },
    spacing: { xs: '0.375rem', sm: '0.75rem', md: '1.25rem', lg: '1.75rem', xl: '2.25rem', '2xl': '3.5rem' },
    borderRadius: { none: '0', sm: '0.75rem', md: '1.25rem', lg: '1.75rem', xl: '2.25rem', full: '9999px' },
    shadows: {
      sm: '0 2px 4px rgba(45, 80, 22, 0.08)',
      md: '0 4px 8px rgba(45, 80, 22, 0.12)',
      lg: '0 8px 16px rgba(45, 80, 22, 0.15)',
      xl: '0 12px 24px rgba(45, 80, 22, 0.18)',
      '2xl': '0 16px 32px rgba(45, 80, 22, 0.2)',
    },
    animations: { duration: { fast: '250ms', normal: '400ms', slow: '600ms' }, easing: { easeInOut: 'cubic-bezier(0.3, 0.1, 0.4, 0.9)', easeOut: 'cubic-bezier(0.1, 0.6, 0.3, 1)', easeIn: 'cubic-bezier(0.7, 0, 0.8, 0.1)', linear: 'linear' } },
  },
}

// ==================== MASTER THEME REGISTRY ====================

export const ALL_THEMES = {
  light: LIGHT_THEMES,
  dark: DARK_THEMES,
  neon: NEON_THEMES,
  glassmorphic: GLASSMORPHIC_THEMES,
  gradient: GRADIENT_THEMES,
  minimalist: MINIMALIST_THEMES,
  cyberpunk: CYBERPUNK_THEMES,
  organic: ORGANIC_THEMES,
}

export const getAllThemes = (): Theme[] => {
  const themes: Theme[] = []
  Object.values(ALL_THEMES).forEach((categoryThemes) => {
    themes.push(...Object.values(categoryThemes))
  })
  return themes
}

export const getThemeById = (id: string): Theme | undefined => {
  for (const categoryThemes of Object.values(ALL_THEMES)) {
    if (id in categoryThemes) {
      return categoryThemes[id]
    }
  }
  return undefined
}

export const getThemesByCategory = (category: string): Theme[] => {
  const categoryThemes = ALL_THEMES[category as keyof typeof ALL_THEMES]
  return categoryThemes ? Object.values(categoryThemes) : []
}

// ==================== THEME STATISTICS ====================

export const THEME_STATISTICS = {
  totalThemes: getAllThemes().length,
  categories: Object.keys(ALL_THEMES).length,
  lightThemes: Object.keys(LIGHT_THEMES).length,
  darkThemes: Object.keys(DARK_THEMES).length,
  neonThemes: Object.keys(NEON_THEMES).length,
  glassmorphicThemes: Object.keys(GLASSMORPHIC_THEMES).length,
  gradientThemes: Object.keys(GRADIENT_THEMES).length,
  minimalistThemes: Object.keys(MINIMALIST_THEMES).length,
  cyberpunkThemes: Object.keys(CYBERPUNK_THEMES).length,
  organicThemes: Object.keys(ORGANIC_THEMES).length,
}

export default {
  getAllThemes,
  getThemeById,
  getThemesByCategory,
  THEME_STATISTICS,
}
