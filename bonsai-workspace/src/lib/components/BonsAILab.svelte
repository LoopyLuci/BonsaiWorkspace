<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'; import { onMount, onDestroy } from 'svelte';
  let prompt='', bonsaiAdapter='', referenceModel='', result:any=null, loading=false, history:any[]=[], activeTab='compare', loopRunning=false, loop:any=null, loopInterval:any;
  async function compare(){ loading=true; try{ result=await invoke('compare_models',{bonsaiAdapter,referenceModelPath:referenceModel,prompt}); }catch(e){ result={error:String(e)}; } finally{ loading=false; } }
  async function loadHistory(){ history=await invoke('get_training_history',{limit:20}); }
  async function toggleLoop(){ if(loopRunning){ await invoke('stop_training_loop'); loopRunning=false; clearInterval(loopInterval); }else{ await invoke('start_training_loop'); loopRunning=true; loopInterval=setInterval(async()=>{ loop=await invoke('get_training_loop_status'); },2000); } }
  onMount(loadHistory); onDestroy(()=>clearInterval(loopInterval));
</script>
<div class="p-4 bg-gray-900 rounded-lg border border-gray-700">
  <div class="flex gap-4 mb-4 text-sm">
    <button class="px-3 py-1 rounded {activeTab==='compare'?'bg-blue-600 text-white':'bg-gray-700 text-gray-400'}" on:click={()=>activeTab='compare'}>Compare</button>
    <button class="px-3 py-1 rounded {activeTab==='loop'?'bg-blue-600 text-white':'bg-gray-700 text-gray-400'}" on:click={()=>activeTab='loop'}>Training Loop</button>
    <button class="px-3 py-1 rounded {activeTab==='history'?'bg-blue-600 text-white':'bg-gray-700 text-gray-400'}" on:click={()=>{activeTab='history';loadHistory();}}>History</button>
  </div>
  {#if activeTab==='compare'}
    <div class="grid grid-cols-2 gap-4 mb-4"><input class="bg-gray-800 border border-gray-700 rounded px-3 py-2 text-white text-sm" placeholder="Adapter path" bind:value={bonsaiAdapter}/><input class="bg-gray-800 border border-gray-700 rounded px-3 py-2 text-white text-sm" placeholder="Reference model" bind:value={referenceModel}/></div>
    <textarea class="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-white text-sm mb-4" rows="3" bind:value={prompt}></textarea>
    <button class="px-4 py-2 bg-blue-600 text-white rounded text-sm" on:click={compare} disabled={loading}>{loading?'Comparing...':'Compare'}</button>
    {#if result}<div class="grid grid-cols-2 gap-4 text-sm mt-4"><div class="bg-gray-800 p-3 rounded"><div class="text-blue-400 font-medium">BonsAI</div><div class="text-gray-300 mt-1">{result.bonsai?.content??result.error}</div></div><div class="bg-gray-800 p-3 rounded"><div class="text-green-400 font-medium">Reference</div><div class="text-gray-300 mt-1">{result.reference?.content??''}</div></div></div>
    <div class="mt-2 text-xs text-gray-400">Intent: {result.intent_match} | Tools: {result.tool_overlap_pct?.toFixed(0)}% | Gaps: {result.gaps?.length??0}</div>{/if}
  {:else if activeTab==='loop'}
    <button class="px-4 py-2 text-sm rounded {loopRunning?'bg-red-600':'bg-green-600'} text-white mb-4" on:click={toggleLoop}>{loopRunning?'Stop':'Start'} Loop</button>
    {#if loop}<div class="grid grid-cols-4 gap-2 text-xs text-gray-400"><div>Rounds: {loop.rounds}</div><div>Examples: {loop.examples_generated}</div><div>Overlap: {loop.avg_overlap_pct?.toFixed(1)}%</div><div>Elapsed: {loop.elapsed_secs}s</div></div>
      <div class="mt-2 h-20 bg-gray-800 rounded overflow-hidden"><canvas id="gapWaterfall" width={400} height={80}></canvas></div>{/if}
  {:else}
    <div class="space-y-2 max-h-80 overflow-y-auto">
      {#each history as run}<div class="bg-gray-800 p-2 rounded text-xs"><span class="text-white">{run.adapter_path}</span> <span class="text-gray-500">{new Date(run.started_at).toLocaleDateString()}</span> <span class="text-green-400">F1:{run.tool_f1?.toFixed(2)}</span></div>{/each}
    </div>
  {/if}
</div>
