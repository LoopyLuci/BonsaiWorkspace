import React from 'react'

export const UninstallButton: React.FC<{ [key: string]: any }> = (props) => (
  <div style={ padding: '1rem', border: '1px solid #E0E0E0', borderRadius: '0.5rem', ...props.style }>
    UninstallButton
  </div>
)

export default UninstallButton
