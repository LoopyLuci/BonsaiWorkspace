import React from 'react'

export const NavigationArrows: React.FC<{ [key: string]: any }> = (props) => (
  <div style={ padding: '1rem', border: '1px solid #E0E0E0', borderRadius: '0.5rem', ...props.style }>
    NavigationArrows
  </div>
)

export default NavigationArrows
