import React from 'react'

export const HelpButton: React.FC<{ onClick?: () => void; children?: React.ReactNode }> = ({ onClick, children = "Help" }) => (
  <button
    onClick={onClick}
    style={{
      padding: '0.75rem 1rem',
      backgroundColor: '#00B0FF',
      color: '#FFFFFF',
      border: 'none',
      borderRadius: '0.5rem',
      cursor: 'pointer',
      fontWeight: 600,
    }}
  >
    {children}
  </button>
)

export default HelpButton
