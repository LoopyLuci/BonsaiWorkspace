import React from 'react'

export const CoordinatesInput: React.FC<{ [key: string]: any }> = (props) => (
  <div style={ padding: '1rem', border: '1px solid #E0E0E0', borderRadius: '0.5rem', ...props.style }>
    CoordinatesInput
  </div>
)

export default CoordinatesInput
