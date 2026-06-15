import React from 'react'

export const DataContext: React.FC<{ [key: string]: any }> = (props) => {
  const style = { padding: '1rem', border: '1px solid #E0E0E0', borderRadius: '0.5rem', ...props.style };
  return (
    <div style={style}>
      DataContext
    </div>
  );
}

export default DataContext
