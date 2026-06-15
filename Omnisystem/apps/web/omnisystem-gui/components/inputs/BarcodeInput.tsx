import React from 'react'

export const BarcodeInput: React.FC<{ [key: string]: any }> = (props) => (
  <div style={ padding: '1rem', border: '1px solid #E0E0E0', borderRadius: '0.5rem', ...props.style }>
    BarcodeInput
  </div>
)

export default BarcodeInput
