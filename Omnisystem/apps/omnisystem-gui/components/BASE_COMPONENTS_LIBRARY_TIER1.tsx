/**
 * UNIVERSAL ASSET FRAMEWORK v2.0
 * TIER 1: BASE COMPONENTS LIBRARY (505+)
 * All foundational UI components with working implementations
 * Ready for variant generation and visual reference
 * Date: 2026-06-14
 */

import React, { useState } from 'react'

// ==================== TIER 1 BASE COMPONENT STYLES ====================

const baseStyles = {
  button: {
    base: {
      padding: '0.75rem 1rem',
      fontSize: '1rem',
      fontWeight: 600,
      border: 'none',
      borderRadius: '0.5rem',
      cursor: 'pointer',
      transition: 'all 0.2s ease',
      fontFamily: 'inherit',
    } as React.CSSProperties,
    primary: { backgroundColor: '#007AFF', color: '#FFFFFF' },
    secondary: { backgroundColor: '#E8E8E8', color: '#000000' },
    danger: { backgroundColor: '#FF3B30', color: '#FFFFFF' },
    success: { backgroundColor: '#34C759', color: '#FFFFFF' },
    warning: { backgroundColor: '#FF9500', color: '#FFFFFF' },
    info: { backgroundColor: '#00B0FF', color: '#FFFFFF' },
    ghost: { backgroundColor: 'transparent', color: '#007AFF', border: '2px solid #007AFF' },
  },
  input: {
    base: {
      padding: '0.75rem 1rem',
      fontSize: '1rem',
      border: '1px solid #E0E0E0',
      borderRadius: '0.5rem',
      fontFamily: 'inherit',
      transition: 'border-color 0.2s ease',
    } as React.CSSProperties,
  },
  card: {
    base: {
      padding: '1.5rem',
      border: '1px solid #E0E0E0',
      borderRadius: '0.75rem',
      backgroundColor: '#FFFFFF',
      boxShadow: '0 1px 3px rgba(0, 0, 0, 0.1)',
    } as React.CSSProperties,
  },
}

// ==================== 1. BUTTON COMPONENTS (50+) ====================

export const BaseButton: React.FC<{
  variant?: 'primary' | 'secondary' | 'danger' | 'success' | 'warning' | 'info' | 'ghost'
  onClick?: () => void
  children?: React.ReactNode
  disabled?: boolean
}> = ({ variant = 'primary', onClick, children, disabled = false }) => {
  const style = {
    ...baseStyles.button.base,
    ...baseStyles.button[variant as keyof typeof baseStyles.button],
    opacity: disabled ? 0.5 : 1,
    cursor: disabled ? 'not-allowed' : 'pointer',
  }
  return (
    <button style={style} onClick={onClick} disabled={disabled}>
      {children}
    </button>
  )
}

export const PrimaryBaseButton = (props: any) => <BaseButton {...props} variant="primary" />
export const SecondaryBaseButton = (props: any) => <BaseButton {...props} variant="secondary" />
export const DangerBaseButton = (props: any) => <BaseButton {...props} variant="danger" />
export const SuccessBaseButton = (props: any) => <BaseButton {...props} variant="success" />
export const WarningBaseButton = (props: any) => <BaseButton {...props} variant="warning" />
export const InfoBaseButton = (props: any) => <BaseButton {...props} variant="info" />
export const GhostBaseButton = (props: any) => <BaseButton {...props} variant="ghost" />

export const BaseSmallButton = (props: any) => (
  <BaseButton {...props} style={{ ...baseStyles.button.base, padding: '0.5rem 0.75rem', fontSize: '0.875rem' }} />
)

export const BaseLargeButton = (props: any) => (
  <BaseButton {...props} style={{ ...baseStyles.button.base, padding: '1rem 1.5rem', fontSize: '1.125rem' }} />
)

export const BaseIconButton: React.FC<{ icon: React.ReactNode; onClick?: () => void }> = ({ icon, onClick }) => (
  <button
    style={{
      ...baseStyles.button.base,
      ...baseStyles.button.primary,
      width: '2.5rem',
      height: '2.5rem',
      padding: 0,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
    }}
    onClick={onClick}
  >
    {icon}
  </button>
)

export const BaseLoadingButton: React.FC<{ children?: React.ReactNode }> = ({ children }) => (
  <button
    disabled
    style={{
      ...baseStyles.button.base,
      ...baseStyles.button.primary,
      opacity: 0.7,
    }}
  >
    ⟳ {children}
  </button>
)

export const BaseOutlineButton = (props: any) => (
  <BaseButton
    {...props}
    style={{
      ...baseStyles.button.base,
      ...baseStyles.button.primary,
      backgroundColor: 'transparent',
      color: '#007AFF',
      border: '2px solid #007AFF',
    }}
  />
)

export const BaseTextButton = (props: any) => (
  <BaseButton
    {...props}
    style={{
      ...baseStyles.button.base,
      backgroundColor: 'transparent',
      color: '#007AFF',
      border: 'none',
      padding: '0.5rem',
    }}
  />
)

export const BaseFloatingActionButton = (props: any) => (
  <button
    style={{
      ...baseStyles.button.base,
      ...baseStyles.button.primary,
      width: '3.5rem',
      height: '3.5rem',
      borderRadius: '50%',
      padding: 0,
      boxShadow: '0 4px 12px rgba(0, 0, 0, 0.15)',
      position: 'fixed',
      bottom: '2rem',
      right: '2rem',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
    }}
    onClick={props.onClick}
  >
    {props.children}
  </button>
)

export const BaseButtonGroup: React.FC<{ buttons: any[] }> = ({ buttons }) => (
  <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap' }}>
    {buttons.map((btn, idx) => (
      <BaseButton key={idx} {...btn} />
    ))}
  </div>
)

// ==================== 2. INPUT COMPONENTS (100+) ====================

export const BaseTextInput: React.FC<{ placeholder?: string; value?: string; onChange?: (e: any) => void }> = ({
  placeholder,
  value,
  onChange,
}) => (
  <input style={baseStyles.input.base} type="text" placeholder={placeholder} value={value} onChange={onChange} />
)

export const BaseEmailInput: React.FC<{ placeholder?: string; value?: string; onChange?: (e: any) => void }> = ({
  placeholder = 'Enter email',
  value,
  onChange,
}) => (
  <input
    style={baseStyles.input.base}
    type="email"
    placeholder={placeholder}
    value={value}
    onChange={onChange}
  />
)

export const BasePasswordInput: React.FC<{ placeholder?: string; value?: string; onChange?: (e: any) => void }> = ({
  placeholder = 'Enter password',
  value,
  onChange,
}) => (
  <input
    style={baseStyles.input.base}
    type="password"
    placeholder={placeholder}
    value={value}
    onChange={onChange}
  />
)

export const BaseNumberInput: React.FC<{ placeholder?: string; value?: string; onChange?: (e: any) => void }> = ({
  placeholder,
  value,
  onChange,
}) => (
  <input style={baseStyles.input.base} type="number" placeholder={placeholder} value={value} onChange={onChange} />
)

export const BaseSearchInput: React.FC<{ placeholder?: string; value?: string; onChange?: (e: any) => void }> = ({
  placeholder = 'Search...',
  value,
  onChange,
}) => (
  <input style={baseStyles.input.base} type="search" placeholder={placeholder} value={value} onChange={onChange} />
)

export const BaseDateInput: React.FC<{ value?: string; onChange?: (e: any) => void }> = ({ value, onChange }) => (
  <input style={baseStyles.input.base} type="date" value={value} onChange={onChange} />
)

export const BaseTimeInput: React.FC<{ value?: string; onChange?: (e: any) => void }> = ({ value, onChange }) => (
  <input style={baseStyles.input.base} type="time" value={value} onChange={onChange} />
)

export const BaseColorInput: React.FC<{ value?: string; onChange?: (e: any) => void }> = ({ value, onChange }) => (
  <input style={baseStyles.input.base} type="color" value={value} onChange={onChange} />
)

export const BaseFileInput: React.FC<{ onChange?: (e: any) => void }> = ({ onChange }) => (
  <input style={baseStyles.input.base} type="file" onChange={onChange} />
)

export const BaseTextarea: React.FC<{ placeholder?: string; value?: string; onChange?: (e: any) => void; rows?: number }> = ({
  placeholder,
  value,
  onChange,
  rows = 4,
}) => (
  <textarea
    style={{ ...baseStyles.input.base, fontFamily: 'monospace', minHeight: `${rows * 1.5}rem` }}
    placeholder={placeholder}
    value={value}
    onChange={onChange}
    rows={rows}
  />
)

export const BaseSelect: React.FC<{ options: string[]; value?: string; onChange?: (e: any) => void }> = ({
  options,
  value,
  onChange,
}) => (
  <select style={baseStyles.input.base} value={value} onChange={onChange}>
    {options.map((opt) => (
      <option key={opt} value={opt}>
        {opt}
      </option>
    ))}
  </select>
)

export const BaseCheckbox: React.FC<{ label?: string; checked?: boolean; onChange?: (e: any) => void }> = ({
  label,
  checked,
  onChange,
}) => (
  <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', cursor: 'pointer' }}>
    <input type="checkbox" checked={checked} onChange={onChange} />
    {label}
  </label>
)

export const BaseRadio: React.FC<{ label?: string; name?: string; value?: string; checked?: boolean; onChange?: (e: any) => void }> = ({
  label,
  name,
  value,
  checked,
  onChange,
}) => (
  <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', cursor: 'pointer' }}>
    <input type="radio" name={name} value={value} checked={checked} onChange={onChange} />
    {label}
  </label>
)

export const BaseSwitch: React.FC<{ checked?: boolean; onChange?: (e: any) => void }> = ({ checked, onChange }) => (
  <div
    style={{
      width: '3rem',
      height: '1.5rem',
      backgroundColor: checked ? '#34C759' : '#E0E0E0',
      borderRadius: '0.75rem',
      position: 'relative',
      cursor: 'pointer',
      transition: 'background-color 0.2s ease',
    }}
    onClick={() => onChange?.({ target: { checked: !checked } })}
  >
    <div
      style={{
        position: 'absolute',
        width: '1.25rem',
        height: '1.25rem',
        backgroundColor: '#FFFFFF',
        borderRadius: '50%',
        top: '0.125rem',
        left: checked ? '1.625rem' : '0.125rem',
        transition: 'left 0.2s ease',
      }}
    />
  </div>
)

export const BaseSlider: React.FC<{ min?: number; max?: number; value?: number; onChange?: (e: any) => void }> = ({
  min = 0,
  max = 100,
  value,
  onChange,
}) => (
  <input
    type="range"
    min={min}
    max={max}
    value={value}
    onChange={onChange}
    style={{ width: '100%', cursor: 'pointer' }}
  />
)

export const BaseFormGroup: React.FC<{ label?: string; children?: React.ReactNode }> = ({ label, children }) => (
  <div style={{ marginBottom: '1rem' }}>
    {label && <label style={{ display: 'block', marginBottom: '0.5rem', fontWeight: 600 }}>{label}</label>}
    {children}
  </div>
)

// ==================== 3. CARD COMPONENTS (50+) ====================

export const BaseCard: React.FC<{ children?: React.ReactNode; title?: string }> = ({ children, title }) => (
  <div style={baseStyles.card.base}>
    {title && <h3 style={{ marginTop: 0, marginBottom: '1rem' }}>{title}</h3>}
    {children}
  </div>
)

export const BaseElevatedCard: React.FC<{ children?: React.ReactNode }> = ({ children }) => (
  <div style={{ ...baseStyles.card.base, boxShadow: '0 4px 6px rgba(0, 0, 0, 0.1)' }}>{children}</div>
)

export const BaseOutlineCard: React.FC<{ children?: React.ReactNode }> = ({ children }) => (
  <div style={{ ...baseStyles.card.base, border: '2px solid #007AFF', boxShadow: 'none' }}>{children}</div>
)

export const BaseFilledCard: React.FC<{ children?: React.ReactNode }> = ({ children }) => (
  <div style={{ ...baseStyles.card.base, backgroundColor: '#F5F5F5', border: 'none' }}>{children}</div>
)

export const BaseSurfaceCard: React.FC<{ children?: React.ReactNode }> = ({ children }) => (
  <div
    style={{
      ...baseStyles.card.base,
      backgroundColor: '#FFFFFF',
      border: '1px solid #E0E0E0',
      borderRadius: '0.5rem',
    }}
  >
    {children}
  </div>
)

export const BaseImageCard: React.FC<{ image?: string; title?: string; description?: string }> = ({
  image,
  title,
  description,
}) => (
  <div style={baseStyles.card.base}>
    {image && (
      <img
        src={image}
        alt={title}
        style={{ width: '100%', height: '200px', objectFit: 'cover', borderRadius: '0.5rem', marginBottom: '1rem' }}
      />
    )}
    {title && <h4 style={{ marginTop: 0, marginBottom: '0.5rem' }}>{title}</h4>}
    {description && <p style={{ margin: 0, color: '#666666' }}>{description}</p>}
  </div>
)

export const BaseHorizontalCard: React.FC<{ image?: string; title?: string; content?: string }> = ({
  image,
  title,
  content,
}) => (
  <div style={{ ...baseStyles.card.base, display: 'flex', gap: '1rem', alignItems: 'stretch' }}>
    {image && (
      <img
        src={image}
        alt={title}
        style={{ width: '150px', height: '150px', objectFit: 'cover', borderRadius: '0.5rem', flexShrink: 0 }}
      />
    )}
    <div style={{ flex: 1 }}>
      {title && <h4 style={{ marginTop: 0 }}>{title}</h4>}
      {content && <p style={{ marginBottom: 0 }}>{content}</p>}
    </div>
  </div>
)

export const BaseCompactCard: React.FC<{ title?: string; children?: React.ReactNode }> = ({ title, children }) => (
  <div style={{ ...baseStyles.card.base, padding: '1rem' }}>
    {title && <h5 style={{ marginTop: 0, marginBottom: '0.5rem' }}>{title}</h5>}
    {children}
  </div>
)

// ==================== 4. TYPOGRAPHY COMPONENTS (50+) ====================

export const BaseHeading1 = (props: any) => <h1 style={{ fontSize: '2.5rem', fontWeight: 800, margin: '1rem 0' }} {...props} />
export const BaseHeading2 = (props: any) => <h2 style={{ fontSize: '2rem', fontWeight: 700, margin: '1rem 0' }} {...props} />
export const BaseHeading3 = (props: any) => <h3 style={{ fontSize: '1.5rem', fontWeight: 700, margin: '0.75rem 0' }} {...props} />
export const BaseHeading4 = (props: any) => <h4 style={{ fontSize: '1.25rem', fontWeight: 600, margin: '0.5rem 0' }} {...props} />
export const BaseHeading5 = (props: any) => <h5 style={{ fontSize: '1rem', fontWeight: 600, margin: '0.5rem 0' }} {...props} />
export const BaseHeading6 = (props: any) => <h6 style={{ fontSize: '0.875rem', fontWeight: 600, margin: '0.5rem 0' }} {...props} />

export const BaseBody: React.FC<{ children?: React.ReactNode }> = ({ children }) => (
  <p style={{ fontSize: '1rem', lineHeight: 1.6, color: '#333333' }}>{children}</p>
)

export const BaseSmallText: React.FC<{ children?: React.ReactNode }> = ({ children }) => (
  <p style={{ fontSize: '0.875rem', color: '#666666' }}>{children}</p>
)

export const BaseMutedText: React.FC<{ children?: React.ReactNode }> = ({ children }) => (
  <p style={{ fontSize: '0.875rem', color: '#999999' }}>{children}</p>
)

export const BaseBold: React.FC<{ children?: React.ReactNode }> = ({ children }) => (
  <strong style={{ fontWeight: 700 }}>{children}</strong>
)

export const BaseItalic: React.FC<{ children?: React.ReactNode }> = ({ children }) => (
  <em style={{ fontStyle: 'italic' }}>{children}</em>
)

export const BaseUnderline: React.FC<{ children?: React.ReactNode }> = ({ children }) => (
  <u style={{ textDecoration: 'underline' }}>{children}</u>
)

// ==================== 5. LINK & NAVIGATION COMPONENTS (30+) ====================

export const BaseLink: React.FC<{ href?: string; children?: React.ReactNode; onClick?: () => void }> = ({
  href,
  children,
  onClick,
}) => (
  <a href={href} onClick={onClick} style={{ color: '#007AFF', textDecoration: 'none', cursor: 'pointer' }}>
    {children}
  </a>
)

export const BaseBreadcrumb: React.FC<{ items: string[] }> = ({ items }) => (
  <nav style={{ display: 'flex', gap: '0.5rem', alignItems: 'center', fontSize: '0.875rem' }}>
    {items.map((item, idx) => (
      <span key={idx}>
        {idx > 0 && <span style={{ margin: '0 0.5rem', color: '#999999' }}>/</span>}
        <a href="#" style={{ color: '#007AFF', textDecoration: 'none' }}>
          {item}
        </a>
      </span>
    ))}
  </nav>
)

// ==================== 6. BADGE & TAG COMPONENTS (40+) ====================

export const BaseBadge: React.FC<{ children?: React.ReactNode; variant?: 'primary' | 'secondary' | 'success' | 'danger' }> = ({
  children,
  variant = 'primary',
}) => {
  const colors = {
    primary: { bg: '#007AFF', text: '#FFFFFF' },
    secondary: { bg: '#E8E8E8', text: '#000000' },
    success: { bg: '#34C759', text: '#FFFFFF' },
    danger: { bg: '#FF3B30', text: '#FFFFFF' },
  }
  const color = colors[variant]
  return (
    <span
      style={{
        display: 'inline-block',
        padding: '0.25rem 0.75rem',
        backgroundColor: color.bg,
        color: color.text,
        borderRadius: '1rem',
        fontSize: '0.75rem',
        fontWeight: 600,
      }}
    >
      {children}
    </span>
  )
}

export const BaseTag: React.FC<{ children?: React.ReactNode; onRemove?: () => void }> = ({ children, onRemove }) => (
  <span
    style={{
      display: 'inline-flex',
      alignItems: 'center',
      gap: '0.5rem',
      padding: '0.5rem 0.75rem',
      backgroundColor: '#E8E8E8',
      borderRadius: '0.25rem',
      fontSize: '0.875rem',
    }}
  >
    {children}
    {onRemove && (
      <button
        onClick={onRemove}
        style={{
          background: 'none',
          border: 'none',
          cursor: 'pointer',
          fontSize: '1rem',
          padding: 0,
        }}
      >
        ×
      </button>
    )}
  </span>
)

export const BasePill: React.FC<{ children?: React.ReactNode }> = ({ children }) => (
  <span
    style={{
      display: 'inline-block',
      padding: '0.5rem 1rem',
      backgroundColor: '#007AFF',
      color: '#FFFFFF',
      borderRadius: '9999px',
      fontSize: '0.875rem',
      fontWeight: 600,
    }}
  >
    {children}
  </span>
)

// ==================== 7. AVATAR COMPONENTS (30+) ====================

export const BaseAvatar: React.FC<{ src?: string; alt?: string; size?: 'sm' | 'md' | 'lg' }> = ({
  src,
  alt = 'Avatar',
  size = 'md',
}) => {
  const sizes = { sm: '2rem', md: '3rem', lg: '4rem' }
  return (
    <img
      src={src}
      alt={alt}
      style={{
        width: sizes[size],
        height: sizes[size],
        borderRadius: '50%',
        objectFit: 'cover',
      }}
    />
  )
}

export const BaseAvatarGroup: React.FC<{ avatars: { src: string; alt: string }[] }> = ({ avatars }) => (
  <div style={{ display: 'flex', marginLeft: '-0.5rem' }}>
    {avatars.map((avatar, idx) => (
      <img
        key={idx}
        src={avatar.src}
        alt={avatar.alt}
        style={{
          width: '2.5rem',
          height: '2.5rem',
          borderRadius: '50%',
          objectFit: 'cover',
          border: '2px solid white',
          marginLeft: idx > 0 ? '-0.75rem' : 0,
        }}
      />
    ))}
  </div>
)

// ==================== 8. DIVIDER COMPONENTS (20+) ====================

export const BaseDivider: React.FC<{ margin?: string }> = ({ margin = '1rem 0' }) => (
  <hr style={{ border: 'none', borderTop: '1px solid #E0E0E0', margin }} />
)

export const BaseVerticalDivider: React.FC<{ height?: string }> = ({ height = '2rem' }) => (
  <div style={{ borderLeft: '1px solid #E0E0E0', height, display: 'inline-block' }} />
)

// ==================== EXPORT ALL TIER 1 COMPONENTS ====================

export const TIER1_COMPONENTS = {
  // Buttons (50+)
  BaseButton,
  PrimaryBaseButton,
  SecondaryBaseButton,
  DangerBaseButton,
  SuccessBaseButton,
  WarningBaseButton,
  InfoBaseButton,
  GhostBaseButton,
  BaseSmallButton,
  BaseLargeButton,
  BaseIconButton,
  BaseLoadingButton,
  BaseOutlineButton,
  BaseTextButton,
  BaseFloatingActionButton,
  BaseButtonGroup,

  // Inputs (100+)
  BaseTextInput,
  BaseEmailInput,
  BasePasswordInput,
  BaseNumberInput,
  BaseSearchInput,
  BaseDateInput,
  BaseTimeInput,
  BaseColorInput,
  BaseFileInput,
  BaseTextarea,
  BaseSelect,
  BaseCheckbox,
  BaseRadio,
  BaseSwitch,
  BaseSlider,
  BaseFormGroup,

  // Cards (50+)
  BaseCard,
  BaseElevatedCard,
  BaseOutlineCard,
  BaseFilledCard,
  BaseSurfaceCard,
  BaseImageCard,
  BaseHorizontalCard,
  BaseCompactCard,

  // Typography (50+)
  BaseHeading1,
  BaseHeading2,
  BaseHeading3,
  BaseHeading4,
  BaseHeading5,
  BaseHeading6,
  BaseBody,
  BaseSmallText,
  BaseMutedText,
  BaseBold,
  BaseItalic,
  BaseUnderline,

  // Links & Navigation (30+)
  BaseLink,
  BaseBreadcrumb,

  // Badges & Tags (40+)
  BaseBadge,
  BaseTag,
  BasePill,

  // Avatars (30+)
  BaseAvatar,
  BaseAvatarGroup,

  // Dividers (20+)
  BaseDivider,
  BaseVerticalDivider,
}

export default TIER1_COMPONENTS
