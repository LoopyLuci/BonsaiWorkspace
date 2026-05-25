<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'; import { onMount, onDestroy } from 'svelte';
  let prompt=''; let bonsaiAdapter=''; let referenceModel=''; let result:any=null; let loading=false; let loopRunning=false; let loopStatus:any=null; let poll:any;
  async function compare(){ loading=true; try{ result=await invoke('compare_models',{bonsaiAdapter,referenceModelPath:referenceModel,prompt}); }catch(e){ result={error:String(e)}; } finally{ loading=false; } }
  async function toggleLoop(){ if(loopRunning){ await invoke('stop_training_loop'); loopRunning=false; clearInterval(poll); }else{ await invoke('start_training_loop'); loopRunning=true; poll=setInterval(async()=>{ loopStatus=await invoke('get_training_loop_status'); },2000); } }
  onDestroy(()=>clearInterval(poll));
</script>
<div class="p-4 bg-gray-900 rounded-lg border border-gray-700">
  <h2 class="text-lg font-semibold text-white mb-4">🧠 BonsAI Lab</h2>
  <div class="grid grid-cols-2 gap-4 mb-4">
    <input class="bg-gray-800 border border-gray-700 rounded px-3 py-2 text-white text-sm" placeholder="BonsAI adapter path" bind:value={bonsaiAdapter}/>
    <input class="bg-gray-800 border border-gray-700 rounded px-3 py-2 text-white text-sm" placeholder="Reference model path" bind:value={referenceModel}/>
  </div>
  <textarea class="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-white text-sm mb-4" rows="3" placeholder="Prompt..." bind:value={prompt}></textarea>
  <div class="flex gap-2 mb-4">
    <button class="px-4 py-2 bg-blue-600 text-white rounded text-sm" on:click={compare} disabled={loading}>{loading?'Comparing...':'Compare'}</button>
    <button class="px-4 py-2 text-sm rounded {loopRunning?'bg-red-600':'bg-green-600'} text-white" on:click={toggleLoop}>{loopRunning?'Stop Loop':'Start Loop'}</button>
  </div>
  {#if loopStatus}<div class="text-xs text-gray-400 mb-4">Rounds: {loopStatus.rounds} | Examples: {loopStatus.examples_generated} | Overlap: {loopStatus.avg_overlap_pct?.toFixed(1)}%</div>{/if}
  {#if result}
    <div class="grid grid-cols-2 gap-4 text-sm">
      <div class="bg-gray-800 p-3 rounded"><div class="text-blue-400 font-medium">BonsAI</div><div class="text-gray-300 mt-1">{result.bonsai?.content??result.error}</div></div>
      <div class="bg-gray-800 p-3 rounded"><div class="text-green-400 font-medium">Reference</div><div class="text-gray-300 mt-1">{result.reference?.content??''}</div></div>
    </div>
    {#if result.intent_match!==undefined}<div class="mt-2 text-xs text-gray-400">Intent match: {result.intent_match} | Tool overlap: {result.tool_overlap_pct?.toFixed(0)}% | Gaps: {result.gaps?.length??0}</div>{/if}
  {/if}
</div>
