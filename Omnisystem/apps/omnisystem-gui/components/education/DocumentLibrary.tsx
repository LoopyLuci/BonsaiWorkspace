import React from 'react'

export const DocumentLibrary: React.FC<{ [key: string]: any }> = (props) => (
  <div style={ padding: '1rem', border: '1px solid #E0E0E0', borderRadius: '0.5rem', ...props.style }>
    DocumentLibrary
  </div>
)

export default DocumentLibrary
