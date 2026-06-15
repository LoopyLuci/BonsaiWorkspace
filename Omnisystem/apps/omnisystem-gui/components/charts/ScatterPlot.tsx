import React from 'react'

export const ScatterPlot: React.FC<{ [key: string]: any }> = (props) => (
  <div style={ padding: '1rem', border: '1px solid #E0E0E0', borderRadius: '0.5rem', ...props.style }>
    ScatterPlot
  </div>
)

export default ScatterPlot
