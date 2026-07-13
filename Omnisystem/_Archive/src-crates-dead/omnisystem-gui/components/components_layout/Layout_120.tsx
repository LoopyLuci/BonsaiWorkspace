import React from 'react'

export const Layout_120: React.FC<{ [key: string]: any }> = (props) => {
  const style = { padding: '1rem', border: '1px solid #E0E0E0', borderRadius: '0.5rem', ...props.style };
  return (
    <div style={style}>
      Layout_120
    </div>
  );
}

export default Layout_120
