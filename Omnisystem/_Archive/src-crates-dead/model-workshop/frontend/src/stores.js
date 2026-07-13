import { writable, derived } from 'svelte/store';

export const modules = writable([]);
export const datasets = writable([]);
export const trainingJobs = writable([]);
export const wsConnected = writable(false);
export const lastUpdate = writable(null);

export function createWebSocket() {
  const ws = new WebSocket('ws://127.0.0.1:4200/ws');

  ws.onopen = () => {
    wsConnected.set(true);
    console.log('🔌 WebSocket connected');
  };

  ws.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data);
      lastUpdate.set(new Date());

      if (data.type === 'training_update') {
        trainingJobs.update(jobs =>
          jobs.map(j => j.job_id === data.payload.job_id ? { ...j, ...data.payload } : j)
        );
      } else if (data.type === 'job_complete') {
        trainingJobs.update(jobs =>
          jobs.map(j => j.job_id === data.payload.job_id ? { ...j, status: 'completed', progress: 1.0 } : j)
        );
      }
    } catch (e) {
      console.error('WebSocket message error:', e);
    }
  };

  ws.onclose = () => {
    wsConnected.set(false);
    console.log('🔴 WebSocket disconnected');
    setTimeout(createWebSocket, 3000);
  };

  return ws;
}

export async function fetchModules() {
  const res = await fetch('/api/modules');
  const data = await res.json();
  modules.set(data);
  return data;
}

export async function fetchDatasets() {
  const res = await fetch('/api/datasets');
  const data = await res.json();
  datasets.set(data);
  return data;
}

export async function fetchTrainingJobs() {
  const res = await fetch('/api/training/jobs');
  const data = await res.json();
  if (data.jobs) {
    trainingJobs.set(data.jobs);
  }
  return data;
}

export async function createModule(name, description, domains, chunks) {
  const res = await fetch('/api/modules', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name, description, domains, chunks }),
  });
  const data = await res.json();
  await fetchModules();
  return data;
}

export async function deleteModule(id) {
  await fetch(`/api/modules/${id}`, { method: 'DELETE' });
  await fetchModules();
}

export async function startTraining(configPath, stages, gpuCount, datasetId) {
  const res = await fetch('/api/models/build', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ config_path: configPath, stages, gpu_count: gpuCount, dataset_id: datasetId }),
  });
  const data = await res.json();
  trainingJobs.update(j => [...j, data]);
  return data;
}

export async function cancelJob(jobId) {
  await fetch(`/api/models/build/${jobId}`, { method: 'DELETE' });
  trainingJobs.update(j => j.map(job => job.job_id === jobId ? { ...job, status: 'cancelled' } : job));
}

export async function convertModel(inputPath, inputFormat, outputFormat, quantization) {
  const res = await fetch('/api/models/convert', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ input_path: inputPath, input_format: inputFormat, output_format: outputFormat, quantization }),
  });
  return await res.json();
}

export async function quantizeModel(inputPath, quantization) {
  const res = await fetch('/api/models/quantize', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ input_path: inputPath, quantization }),
  });
  return await res.json();
}

export async function designModel(config) {
  const res = await fetch('/api/models/design', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(config),
  });
  return await res.json();
}

export async function validateModelConfig(config) {
  const res = await fetch('/api/models/design/validate', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(config),
  });
  return await res.json();
}
