import type { VisionSnapshot } from '$lib/stores/vision';

type BuildVisionContextParams = {
  userText: string;
  streamActive: boolean;
  snapshot: VisionSnapshot | null;
  frameCaptured: boolean;
  visionAttachmentReady: boolean;
};

const VISION_MODEL_HINT = /(vision|llava|vl\b|qwen2[-_.]?vl|minicpm[-_.]?v|phi[-_.]?3[-_.]?vision|gpt[-_.]?4o|gemini)/i;

export function isLikelyVisionCapableModel(modelIdOrName: string | null | undefined): boolean {
  if (!modelIdOrName) return false;
  return VISION_MODEL_HINT.test(modelIdOrName);
}

export function buildVisionContextMessage(params: BuildVisionContextParams): string | null {
  const { userText, streamActive, snapshot, frameCaptured, visionAttachmentReady } = params;
  if (!streamActive || !snapshot) return null;

  const asksAboutScreen = /(screen|share|shared|display|visible|look at|see what|on my screen)/i.test(userText);
  if (!asksAboutScreen && userText.trim().length < 1) {
    return null;
  }

  return [
    'Realtime collaboration context:',
    'A live screen-share stream is active via Agent Vision telemetry.',
    'Use this context directly. Do not claim there is no screen context available.',
    '',
    `- Captured at: ${snapshot.timestampIso}`,
    `- Resolution: ${snapshot.resolution}`,
    `- Edge density: ${snapshot.edgeDensityPct}%`,
    `- Motion delta: ${snapshot.motionDeltaPct}%`,
    `- Luminance (0-255): ${snapshot.luminance}`,
    `- Frame captured locally: ${frameCaptured ? 'yes' : 'not yet'}`,
    `- Vision attachment path ready: ${visionAttachmentReady ? 'yes (vision-capable route)' : 'metadata-only route currently active'}`,
    '',
    'Important honesty policy: this is telemetry, not a direct image interpretation in the default text-only path.',
    'Do not fabricate exact on-screen text or controls unless the user confirms them.',
    'If details are uncertain, ask a concise follow-up question about the visible UI state.',
  ].join('\n');
}
