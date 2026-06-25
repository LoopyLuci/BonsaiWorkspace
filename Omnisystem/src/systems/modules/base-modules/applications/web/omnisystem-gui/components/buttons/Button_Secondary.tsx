import React from 'react'
export const Button_Secondary: React.FC<{ onClick?: () => void }> = ({ onClick }) => <button onClick={onClick} style={{ padding: '0.75rem 1rem', backgroundColor: '#E8E8E8', color: '#000', border: 'none', borderRadius: '0.5rem', cursor: 'pointer', fontWeight: 600 }}>Secondary</button>
export default Button_Secondary