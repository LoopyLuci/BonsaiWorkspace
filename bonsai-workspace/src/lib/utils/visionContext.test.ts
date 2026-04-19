import { describe, expect, it } from 'vitest';
import { buildVisionContextMessage } from './visionContext';

describe('buildVisionContextMessage', () => {
  const snapshot = {
    timestampIso: '2026-04-17T21:18:32.503Z',
    resolution: '1920x1080',
    edgeDensityPct: 12.4,
    luminance: 126,
    motionDeltaPct: 3.1,
  };

  it('returns null when stream is inactive', () => {
    const message = buildVisionContextMessage({
      userText: 'can you see my screen?',
      streamActive: false,
      snapshot,
    });

    expect(message).toBeNull();
  });

  it('includes telemetry fields when screen context is requested', () => {
    const message = buildVisionContextMessage({
      userText: 'can you see what is on my screen?',
      streamActive: true,
      snapshot,
    });

    expect(message).toContain('Realtime collaboration context:');
    expect(message).toContain('Resolution: 1920x1080');
    expect(message).toContain('Edge density: 12.4%');
    expect(message).toContain('Motion delta: 3.1%');
    expect(message).toContain('Luminance (0-255): 126');
  });
});
