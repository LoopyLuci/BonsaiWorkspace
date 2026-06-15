import React from 'react'

export const ArchiveButton: React.FC<{ onClick?: () => void; children?: React.ReactNode }> = ({ onClick, children = "Archive" }) => (
  <button
    onClick={onClick}
    style={{
      padding: '0.75rem 1rem',
      backgroundColor: '#999999',
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

export default ArchiveButton
