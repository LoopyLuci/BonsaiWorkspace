<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';

  let prompt='', workspaceAdapter='', referenceModel='', result:any=null, loading=false;
  let history:any[]=[], activeTab='compare';
  let loopRunning=false, loop:any=null, loopInterval:any;
  let progressHistory:number[]=[];
  let canvas:HTMLCanvasElement;
  let unlisten:()=>void;

  async function compare(){ loading=true; try{ result=await invoke('compare_models',{workspaceAdapter,referenceModelPath:referenceModel,prompt}); }catch(e){ result={error:String(e)}; } finally{ loading=false; } }
  async function loadHistory(){ history=await invoke('get_training_history',{limit:20}); }

  async function toggleLoop(){
    if(loopRunning){
      await invoke('stop_training_loop'); loopRunning=false; clearInterval(loopInterval);
    } else {
      await invoke('start_training_loop'); loopRunning=true;
      loopInterval=setInterval(async()=>{ loop=await invoke('get_training_loop_status'); },2000);
    }
  }

  function drawWaterfall(){
    if(!canvas) return;
    const ctx=canvas.getContext('2d'); if(!ctx) return;
    const w=canvas.width, h=canvas.height;
    ctx.clearRect(0,0,w,h);
    const bar=Math.max(2, w/Math.max(progressHistory.length,1));
    progressHistory.forEach((v,i)=>{
      const x=i*bar, bh=(v/100)*h;
      ctx.fillStyle=`hsl(${120*v/100},70%,50%)`;
      ctx.fillRect(x,h-bh,bar-1,bh);
    });
  }

  onMount(async()=>{
    await loadHistory();
    unlisten=await listen<any>('training-loop-progress',(ev)=>{
      loop=ev.payload;
      progressHistory=[...progressHistory, ev.payload.tool_overlap_pct??0].slice(-80);
      drawWaterfall();
    });
  });
  onDestroy(()=>{ clearInterval(loopInterval); unlisten?.(); });
</script>

<div class="p-4 bg-gray-900 rounded-lg border border-gray-700">
  <div class="flex gap-4 mb-4 text-sm">
    <button class="px-3 py-1 rounded {activeTab==='compare'?'bg-blue-600 text-white':'bg-gray-700 text-gray-400'}" on:click={()=>activeTab='compare'}>Compare</button>
    <button class="px-3 py-1 rounded {activeTab==='loop'?'bg-blue-600 text-white':'bg-gray-700 text-gray-400'}" on:click={()=>activeTab='loop'}>Training Loop</button>
    <button class="px-3 py-1 rounded {activeTab==='history'?'bg-blue-600 text-white':'bg-gray-700 text-gray-400'}" on:click={()=>{activeTab='history';loadHistory();}}>History</button>
  </div>

  {#if activeTab==='compare'}
    <div class="grid grid-cols-2 gap-4 mb-4">
      <input class="bg-gray-800 border border-gray-700 rounded px-3 py-2 text-white text-sm" placeholder="Adapter path" bind:value={workspaceAdapter}/>
      <input class="bg-gray-800 border border-gray-700 rounded px-3 py-2 text-white text-sm" placeholder="Reference model" bind:value={referenceModel}/>
    </div>
    <textarea class="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-white text-sm mb-4" rows="3" bind:value={prompt}></textarea>
    <button class="px-4 py-2 bg-blue-600 text-white rounded text-sm" on:click={compare} disabled={loading}>{loading?'Comparing...':'Compare'}</button>
    {#if result}
      <div class="grid grid-cols-2 gap-4 text-sm mt-4">
        <div class="bg-gray-800 p-3 rounded"><div class="text-blue-400 font-medium">OmniAI</div><div class="text-gray-300 mt-1">{result.workspace?.content??result.error}</div></div>
        <div class="bg-gray-800 p-3 rounded"><div class="text-green-400 font-medium">Reference</div><div class="text-gray-300 mt-1">{result.reference?.content??''}</div></div>
      </div>
      <div class="mt-2 text-xs text-gray-400">Intent: {result.intent_match} | Tools: {result.tool_overlap_pct?.toFixed(0)}% | Gaps: {result.gaps?.length??0}</div>
    {/if}

  {:else if activeTab==='loop'}
    <button class="px-4 py-2 text-sm rounded {loopRunning?'bg-red-600':'bg-green-600'} text-white mb-4" on:click={toggleLoop}>{loopRunning?'Stop':'Start'} Loop</button>
    {#if loop}
      <div class="grid grid-cols-4 gap-2 text-xs text-gray-400 mb-2">
        <div>Rounds: {loop.rounds??loop.round}</div>
        <div>Examples: {loop.examples_generated??loop.examples}</div>
        <div>Overlap: {loop.tool_overlap_pct?.toFixed(1)}%</div>
        <div>Elapsed: {loop.elapsed_secs}s</div>
      </div>
      <div class="h-20 bg-gray-800 rounded overflow-hidden">
        <canvas bind:this={canvas} width={400} height={80} class="w-full h-full"></canvas>
      </div>
    {/if}

  {:else}
    <div class="space-y-2 max-h-80 overflow-y-auto">
      {#each history as run}
        <div class="bg-gray-800 p-2 rounded text-xs">
          <span class="text-white">{run.adapter_path}</span>
          <span class="text-gray-500 ml-2">{new Date(run.started_at).toLocaleDateString()}</span>
          <span class="text-green-400 ml-2">F1:{run.tool_f1?.toFixed(2)}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>
