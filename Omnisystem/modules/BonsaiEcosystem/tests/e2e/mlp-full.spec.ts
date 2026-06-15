import { test, expect } from '@playwright/test';
const API = 'http://127.0.0.1:11369/api/v1';
test.describe('MLP Full',()=>{
  test('1-chat',async({request})=>{ const r=await request.post(`${API}/chat`,{data:{messages:[{role:'user',content:'2+2?'}]}}); expect(r.ok()).toBeTruthy(); expect((await r.json()).content).toBeTruthy(); });
  test('2-code-agent',async({request})=>{ const r=await request.post(`${API}/agents/message`,{data:{agentId:'code-writer',message:{role:'user',content:'Create hello.py'}}}); expect((await r.json()).actions?.length).toBeGreaterThan(0); });
  test('3-sandbox',async({request})=>{ const r=await request.post(`${API}/sandbox/run`,{data:{code:'print(42)',language:'python',sandbox_tier:'venv',workspace:'.'}}); expect((await r.json()).stdout).toContain('42'); });
  test('4-features-off',async({request})=>{ const r=await request.get(`${API}/features`); const j=await r.json(); expect(j.swarm_enabled??true).toBe(false); });
  test('5-gpu-stats',async({request})=>{ expect((await request.get(`${API}/core/stats`)).ok()).toBeTruthy(); });
});
