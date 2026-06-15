import React from 'react'

export const PartialDrawer: React.FC<{ [key: string]: any }> = (props) => (
  <div style={ padding: '1rem', border: '1px solid #E0E0E0', borderRadius: '0.5rem', ...props.style }>
    PartialDrawer
  </div>
)

export default PartialDrawer
