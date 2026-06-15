import React from 'react'

export const PreviousButton: React.FC<{ onClick?: () => void; children?: React.ReactNode }> = ({ onClick, children = "Previous" }) => (
  <button
    onClick={onClick}
    style={{
      padding: '0.75rem 1rem',
      backgroundColor: '#E8E8E8',
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

export default PreviousButton
