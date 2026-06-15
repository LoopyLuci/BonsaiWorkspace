import React from 'react'

export const RefreshButton: React.FC<{ onClick?: () => void; children?: React.ReactNode }> = ({ onClick, children = "Refresh" }) => (
  <button
    onClick={onClick}
    style={{
      padding: '0.75rem 1rem',
      backgroundColor: '#007AFF',
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

export default RefreshButton
