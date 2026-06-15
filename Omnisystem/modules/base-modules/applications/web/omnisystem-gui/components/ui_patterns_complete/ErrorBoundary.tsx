import React from 'react'

export const ErrorBoundary: React.FC<{ [key: string]: any }> = (props) => (
  <div style={ padding: '1rem', border: '1px solid #E0E0E0', borderRadius: '0.5rem', ...props.style }>
    ErrorBoundary
  </div>
)

export default ErrorBoundary
