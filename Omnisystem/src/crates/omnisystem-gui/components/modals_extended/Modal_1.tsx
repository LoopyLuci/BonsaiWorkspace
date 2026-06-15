import React from 'react'

export const Modal_1: React.FC<{ [key: string]: any }> = (props) => {
  const style = { padding: '1rem', border: '1px solid #E0E0E0', borderRadius: '0.5rem', ...props.style };
  return (
    <div style={style}>
      Modal_1
    </div>
  );
}

export default Modal_1
