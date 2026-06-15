import React from 'react'

export const Input_50: React.FC<{ [key: string]: any }> = (props) => {
  const style = { padding: '1rem', border: '1px solid #E0E0E0', borderRadius: '0.5rem', ...props.style };
  return (
    <div style={style}>
      Input_50
    </div>
  );
}

export default Input_50
