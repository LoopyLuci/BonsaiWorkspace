import React from 'react'

export const NumberInput: React.FC<{ onChange?: (value: string) => void; placeholder?: string }> = ({ onChange, placeholder = "Enter number" }) => (
  <input
    type="number"
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

export default NumberInput
