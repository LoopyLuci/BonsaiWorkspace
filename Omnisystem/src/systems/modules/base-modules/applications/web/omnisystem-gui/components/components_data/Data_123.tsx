import React from 'react'

export const Data_123: React.FC<{ [key: string]: any }> = (props) => {
  const style = { padding: '1rem', border: '1px solid #E0E0E0', borderRadius: '0.5rem', ...props.style };
  return (
    <div style={style}>
      Data_123
    </div>
  );
}

export default Data_123
