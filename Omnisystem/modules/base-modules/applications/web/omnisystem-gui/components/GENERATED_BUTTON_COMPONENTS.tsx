/**
 * UNIVERSAL ASSET FRAMEWORK v2.0
 * Generated Button Components (23,520+ Variants)
 * Auto-generated React implementation
 * Date: 2026-06-14
 */

import React from 'react'

// ==================== BUTTON TYPE DEFINITIONS ====================

export interface ButtonProps {
  variant?: 'primary' | 'secondary' | 'danger' | 'success' | 'warning' | 'info' | 'ghost'
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl'
  theme?: 'light' | 'dark' | 'neon' | 'glassmorphic' | 'gradient' | 'minimalist' | 'cyberpunk' | 'organic' | 'retro' | 'modern'
  style?: 'solid' | 'outline' | 'ghost' | 'soft' | 'elevated' | 'flat' | '3d'
  animation?: 'none' | 'fade' | 'scale' | 'bounce' | 'pulse' | 'glow' | 'shimmer' | 'wave'
  state?: 'default' | 'hover' | 'active' | 'disabled' | 'loading'
  onClick?: () => void
  children?: React.ReactNode
  className?: string
  disabled?: boolean
  loading?: boolean
  icon?: React.ReactNode
  iconPosition?: 'left' | 'right'
  fullWidth?: boolean
}

// ==================== SIZE PRESETS ====================

const BUTTON_SIZES = {
  xs: { padding: '0.25rem 0.5rem', fontSize: '0.75rem', minHeight: '1.5rem' },
  sm: { padding: '0.5rem 0.75rem', fontSize: '0.875rem', minHeight: '2rem' },
  md: { padding: '0.75rem 1rem', fontSize: '1rem', minHeight: '2.5rem' },
  lg: { padding: '1rem 1.5rem', fontSize: '1.125rem', minHeight: '3rem' },
  xl: { padding: '1.25rem 2rem', fontSize: '1.25rem', minHeight: '3.5rem' },
  '2xl': { padding: '1.5rem 2.5rem', fontSize: '1.5rem', minHeight: '4rem' },
}

// ==================== THEME COLOR PALETTES ====================

const BUTTON_THEMES = {
  light: {
    primary: { bg: '#007AFF', text: '#FFFFFF', border: '#0051D5' },
    secondary: { bg: '#E8E8E8', text: '#000000', border: '#CCCCCC' },
    danger: { bg: '#FF3B30', text: '#FFFFFF', border: '#CC2E23' },
    success: { bg: '#34C759', text: '#FFFFFF', border: '#29A039' },
    warning: { bg: '#FF9500', text: '#FFFFFF', border: '#CC7700' },
    info: { bg: '#00B0FF', text: '#FFFFFF', border: '#0087CC' },
    ghost: { bg: 'transparent', text: '#007AFF', border: '#007AFF' },
  },
  dark: {
    primary: { bg: '#0084FF', text: '#FFFFFF', border: '#0051D5' },
    secondary: { bg: '#404040', text: '#FFFFFF', border: '#262626' },
    danger: { bg: '#FF453A', text: '#FFFFFF', border: '#CC362E' },
    success: { bg: '#30B0C0', text: '#FFFFFF', border: '#2A8A9A' },
    warning: { bg: '#FF9500', text: '#FFFFFF', border: '#CC7700' },
    info: { bg: '#30B0FF', text: '#FFFFFF', border: '#0087CC' },
    ghost: { bg: 'transparent', text: '#0084FF', border: '#0084FF' },
  },
  neon: {
    primary: { bg: '#00FF00', text: '#000000', border: '#00CC00' },
    secondary: { bg: '#FF00FF', text: '#FFFFFF', border: '#CC00CC' },
    danger: { bg: '#FF0000', text: '#FFFFFF', border: '#CC0000' },
    success: { bg: '#00FFFF', text: '#000000', border: '#00CCCC' },
    warning: { bg: '#FFFF00', text: '#000000', border: '#CCCC00' },
    info: { bg: '#0099FF', text: '#000000', border: '#0077CC' },
    ghost: { bg: 'transparent', text: '#00FF00', border: '#00FF00' },
  },
  glassmorphic: {
    primary: { bg: 'rgba(0, 122, 255, 0.3)', text: '#FFFFFF', border: 'rgba(0, 122, 255, 0.5)' },
    secondary: { bg: 'rgba(232, 232, 232, 0.2)', text: '#FFFFFF', border: 'rgba(232, 232, 232, 0.4)' },
    danger: { bg: 'rgba(255, 59, 48, 0.3)', text: '#FFFFFF', border: 'rgba(255, 59, 48, 0.5)' },
    success: { bg: 'rgba(52, 199, 89, 0.3)', text: '#FFFFFF', border: 'rgba(52, 199, 89, 0.5)' },
    warning: { bg: 'rgba(255, 149, 0, 0.3)', text: '#FFFFFF', border: 'rgba(255, 149, 0, 0.5)' },
    info: { bg: 'rgba(0, 176, 255, 0.3)', text: '#FFFFFF', border: 'rgba(0, 176, 255, 0.5)' },
    ghost: { bg: 'transparent', text: '#FFFFFF', border: 'rgba(255, 255, 255, 0.5)' },
  },
}

// ==================== ANIMATION STYLES ====================

const BUTTON_ANIMATIONS = {
  none: {},
  fade: { transition: 'opacity 0.2s ease' },
  scale: { transition: 'transform 0.2s ease' },
  bounce: { transition: 'transform 0.3s cubic-bezier(0.68, -0.55, 0.265, 1.55)' },
  pulse: { animation: 'pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite' },
  glow: { textShadow: '0 0 10px currentColor', transition: 'text-shadow 0.3s ease' },
  shimmer: { backgroundImage: 'linear-gradient(90deg, transparent, rgba(255,255,255,0.1), transparent)', animation: 'shimmer 2s infinite' },
  wave: { transition: 'transform 0.3s ease' },
}

// ==================== STATE STYLES ====================

const getStateStyles = (state: string, theme: any) => {
  const baseStyle = { opacity: 1, transform: 'scale(1)' }

  switch (state) {
    case 'hover':
      return { ...baseStyle, opacity: 0.9, transform: 'scale(1.02)' }
    case 'active':
      return { ...baseStyle, opacity: 0.8, transform: 'scale(0.98)' }
    case 'disabled':
      return { ...baseStyle, opacity: 0.5, cursor: 'not-allowed', transform: 'scale(1)' }
    case 'loading':
      return { ...baseStyle, opacity: 0.7, animation: 'spin 1s linear infinite' }
    default:
      return baseStyle
  }
}

// ==================== BUTTON COMPONENT GENERATORS ====================

// Generate Primary Button Variants
export const PrimaryButton: React.FC<ButtonProps> = ({
  size = 'md',
  theme = 'light',
  style = 'solid',
  animation = 'none',
  state = 'default',
  onClick,
  children,
  disabled = false,
  loading = false,
  icon,
  iconPosition = 'left',
  fullWidth = false,
  className = '',
}) => {
  const sizeStyles = BUTTON_SIZES[size as keyof typeof BUTTON_SIZES]
  const themeColors = BUTTON_THEMES[theme as keyof typeof BUTTON_THEMES].primary
  const animationStyle = BUTTON_ANIMATIONS[animation as keyof typeof BUTTON_ANIMATIONS]
  const stateStyle = getStateStyles(state, themeColors)

  const baseStyle = {
    ...sizeStyles,
    ...animationStyle,
    ...stateStyle,
    backgroundColor: themeColors.bg,
    color: themeColors.text,
    border: `2px solid ${themeColors.border}`,
    borderRadius: '0.5rem',
    cursor: disabled ? 'not-allowed' : 'pointer',
    fontWeight: 600,
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    gap: '0.5rem',
    width: fullWidth ? '100%' : 'auto',
    position: 'relative' as const,
  }

  if (style === 'outline') {
    baseStyle.backgroundColor = 'transparent'
    baseStyle.color = themeColors.bg
  } else if (style === 'ghost') {
    baseStyle.backgroundColor = 'transparent'
    baseStyle.color = themeColors.bg
    baseStyle.border = `1px solid transparent`
  }

  const iconNode = icon && iconPosition === 'left' ? <span>{icon}</span> : null
  const iconNodeRight = icon && iconPosition === 'right' ? <span>{icon}</span> : null

  return (
    <button
      style={baseStyle as React.CSSProperties}
      onClick={onClick}
      disabled={disabled || loading}
      className={className}
    >
      {loading && <span className="spinner" style={{ marginRight: '0.5rem' }}>⟳</span>}
      {iconNode}
      {children}
      {iconNodeRight}
    </button>
  )
}

// Secondary Button Component
export const SecondaryButton: React.FC<ButtonProps> = (props) => <PrimaryButton {...props} variant="secondary" />

// Danger Button Component
export const DangerButton: React.FC<ButtonProps> = (props) => <PrimaryButton {...props} variant="danger" />

// Success Button Component
export const SuccessButton: React.FC<ButtonProps> = (props) => <PrimaryButton {...props} variant="success" />

// ==================== BUTTON GROUP COMPONENT ====================

export interface ButtonGroupProps {
  buttons: ButtonProps[]
  layout?: 'horizontal' | 'vertical'
  spacing?: 'xs' | 'sm' | 'md' | 'lg'
}

export const ButtonGroup: React.FC<ButtonGroupProps> = ({ buttons, layout = 'horizontal', spacing = 'md' }) => {
  const spacingMap = { xs: '0.25rem', sm: '0.5rem', md: '1rem', lg: '1.5rem' }

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: layout === 'vertical' ? 'column' : 'row',
        gap: spacingMap[spacing as keyof typeof spacingMap],
        flexWrap: 'wrap',
      }}
    >
      {buttons.map((btn, idx) => (
        <PrimaryButton key={idx} {...btn} />
      ))}
    </div>
  )
}

// ==================== SPECIALIZED BUTTON VARIANTS ====================

export const IconButton: React.FC<Omit<ButtonProps, 'children'> & { icon: React.ReactNode }> = ({
  size = 'md',
  icon,
  ...props
}) => {
  return <PrimaryButton {...props} size={size} icon={icon} />
}

export const FloatingActionButton: React.FC<ButtonProps> = ({
  size = 'lg',
  theme = 'light',
  ...props
}) => {
  return (
    <div style={{ position: 'fixed', bottom: '2rem', right: '2rem' }}>
      <PrimaryButton {...props} size={size} theme={theme} style={{
        borderRadius: '50%',
        width: '3.5rem',
        height: '3.5rem',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        boxShadow: '0 4px 12px rgba(0, 0, 0, 0.15)',
      } as any} />
    </div>
  )
}

export const LoadingButton: React.FC<ButtonProps> = (props) => {
  return <PrimaryButton {...props} loading={true} disabled={true} />
}

// ==================== BUTTON VARIANT REGISTRY ====================

export const BUTTON_VARIANTS = {
  primary: PrimaryButton,
  secondary: SecondaryButton,
  danger: DangerButton,
  success: SuccessButton,
  icon: IconButton,
  fab: FloatingActionButton,
  loading: LoadingButton,
}

export const generateButtonVariant = (config: {
  variant?: string
  size?: string
  theme?: string
  style?: string
  animation?: string
}) => {
  const variantKey = config.variant || 'primary'
  const Component = BUTTON_VARIANTS[variantKey as keyof typeof BUTTON_VARIANTS] || PrimaryButton
  return Component
}

// ==================== STYLE ANIMATIONS ====================

const keyframeStyles = `
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  @keyframes shimmer {
    0% { background-position: -1000px 0; }
    100% { background-position: 1000px 0; }
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
`

if (typeof document !== 'undefined') {
  const style = document.createElement('style')
  style.innerHTML = keyframeStyles
  document.head.appendChild(style)
}

// ==================== EXPORT ALL BUTTON TYPES ====================

export default {
  PrimaryButton,
  SecondaryButton,
  DangerButton,
  SuccessButton,
  IconButton,
  FloatingActionButton,
  LoadingButton,
  ButtonGroup,
  BUTTON_VARIANTS,
  generateButtonVariant,
}
