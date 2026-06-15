import React from 'react'

export const DeleteButton: React.FC<{ onClick?: () => void; children?: React.ReactNode }> = ({ onClick, children = "Delete" }) => (
  <button
    onClick={onClick}
    style={{
      padding: '0.75rem 1rem',
      backgroundColor: '#FF3B30',
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

export default DeleteButton
