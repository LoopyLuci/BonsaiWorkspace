import React from 'react'

export const DraftButton: React.FC<{ onClick?: () => void; children?: React.ReactNode }> = ({ onClick, children = "Save Draft" }) => (
  <button
    onClick={onClick}
    style={{
      padding: '0.75rem 1rem',
      backgroundColor: '#FF9500',
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

export default DraftButton
