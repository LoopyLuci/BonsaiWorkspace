import React from 'react'

export const Button_Primary: React.FC<{ children?: React.ReactNode; onClick?: () => void }> = ({ children, onClick }) => (
  <button
    onClick={onClick}
    style={{
      padding: '0.75rem 1rem',
      fontSize: '1rem',
      backgroundColor: '#007AFF',
      color: '#FFFFFF',
      border: 'none',
      borderRadius: '0.5rem',
      cursor: 'pointer',
      fontWeight: 600,
    }}
  >
    {children || 'Primary'}
  </button>
)

export default Button_Primary
