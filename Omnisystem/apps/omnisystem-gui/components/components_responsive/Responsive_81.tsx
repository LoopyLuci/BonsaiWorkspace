import React from 'react'

export const Responsive_81: React.FC<{ [key: string]: any }> = (props) => {
  const style = { padding: '1rem', border: '1px solid #E0E0E0', borderRadius: '0.5rem', ...props.style };
  return (
    <div style={style}>
      Responsive_81
    </div>
  );
}

export default Responsive_81
