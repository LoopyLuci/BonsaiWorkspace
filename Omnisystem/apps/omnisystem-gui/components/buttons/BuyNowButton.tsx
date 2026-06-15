import React from 'react'

export const BuyNowButton: React.FC<{ onClick?: () => void; children?: React.ReactNode }> = ({ onClick, children = "Buy Now" }) => (
  <button
    onClick={onClick}
    style={{
      padding: '0.75rem 1rem',
      backgroundColor: '#FF6B35',
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

export default BuyNowButton
