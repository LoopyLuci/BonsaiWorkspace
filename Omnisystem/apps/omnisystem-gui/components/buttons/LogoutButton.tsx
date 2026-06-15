import React from 'react'

export const LogoutButton: React.FC<{ onClick?: () => void; children?: React.ReactNode }> = ({ onClick, children = "Logout" }) => (
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

export default LogoutButton
