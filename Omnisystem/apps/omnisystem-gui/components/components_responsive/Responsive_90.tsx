import React from 'react'

export const Responsive_90: React.FC<{ [key: string]: any }> = (props) => {
  const style = { padding: '1rem', border: '1px solid #E0E0E0', borderRadius: '0.5rem', ...props.style };
  return (
    <div style={style}>
      Responsive_90
    </div>
  );
}

export default Responsive_90
