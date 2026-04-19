import { writable } from 'svelte/store';

export interface VisionSnapshot {
  timestampIso: string;
  resolution: string;
  edgeDensityPct: number;
  luminance: number;
  motionDeltaPct: number;
}

export interface VisionFrame {
  timestampIso: string;
  resolution: string;
  dataUrl: string;
}

export const visionStreamActive = writable(false);
export const latestVisionSnapshot = writable<VisionSnapshot | null>(null);
export const latestVisionFrame = writable<VisionFrame | null>(null);

export function setVisionStreamActive(active: boolean) {
  visionStreamActive.set(active);
}

export function upsertVisionSnapshot(snapshot: VisionSnapshot) {
  latestVisionSnapshot.set(snapshot);
}

export function upsertVisionFrame(frame: VisionFrame) {
  latestVisionFrame.set(frame);
}

export function clearVisionSnapshot() {
  latestVisionSnapshot.set(null);
  latestVisionFrame.set(null);
}
