import React from 'react'

export const ColorPicker: React.FC<{ onChange?: (value: string) => void; placeholder?: string }> = ({ onChange, placeholder = "Select color" }) => (
  <input
    type="color"
    onChange={(e) => onChange?.(e.target.value)}
    placeholder={placeholder}
    style={{
      padding: '0.75rem 1rem',
      fontSize: '1rem',
      border: '1px solid #E0E0E0',
      borderRadius: '0.5rem',
      fontFamily: 'inherit',
    }}
  />
)

export default ColorPicker
