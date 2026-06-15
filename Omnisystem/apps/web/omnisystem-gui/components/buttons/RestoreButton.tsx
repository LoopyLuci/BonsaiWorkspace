import React from 'react'

export const RestoreButton: React.FC<{ onClick?: () => void; children?: React.ReactNode }> = ({ onClick, children = "Restore" }) => (
  <button
    onClick={onClick}
    style={{
      padding: '0.75rem 1rem',
      backgroundColor: '#34C759',
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

export default RestoreButton
