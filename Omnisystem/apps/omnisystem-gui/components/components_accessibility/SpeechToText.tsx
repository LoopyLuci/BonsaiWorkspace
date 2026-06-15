import React from 'react'

export const SpeechToText: React.FC<{ [key: string]: any }> = (props) => {
  const style = { padding: '1rem', border: '1px solid #E0E0E0', borderRadius: '0.5rem', ...props.style };
  return (
    <div style={style}>
      SpeechToText
    </div>
  );
}

export default SpeechToText
