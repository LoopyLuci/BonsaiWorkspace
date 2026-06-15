import React from 'react'

export const NetworkError: React.FC<{ [key: string]: any }> = (props) => (
  <div style={ padding: '1rem', border: '1px solid #E0E0E0', borderRadius: '0.5rem', ...props.style }>
    NetworkError
  </div>
)

export default NetworkError
