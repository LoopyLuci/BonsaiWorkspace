/**
 * Widget registry — the procedural-UI entry point for Workspace panels.
 *
 * Adding a new capability to the shell should mean "write a manifest entry
 * here + drop a component file", not editing `App.svelte`'s toolbar state
 * and render blocks by hand (the pattern every existing overlay panel uses
 * today). `WidgetHost.svelte` is the mount point that reads this registry.
 *
 * `entry` is a plain dynamic-import thunk — Vite still statically analyses
 * and code-splits it exactly like the existing hand-written
 * `import('$lib/components/X.svelte').then(...)` calls in `App.svelte`, so
 * this changes nothing about bundling; it just gives every panel a stable
 * id or Vite chunks it. It's what makes the panel loadable/upgradable on
 * its own instead of only as part of the main bundle.
 */

import type { ComponentType, SvelteComponent } from 'svelte';

export interface WidgetManifest {
  id: string;
  title: string;
  icon: string;
  /** Dynamic-import thunk resolving to the widget's default-exported component. */
  entry: () => Promise<{ default: ComponentType<SvelteComponent> }>;
  /**
   * Minimum trust score (0-100, see `capability-registry/trust_score.rs`)
   * required to load this widget at all. Undefined = no gate (the default,
   * built-in widgets). Reserved for agent-authored/remote widgets (see
   * `WidgetHost`'s `loadRemoteWidget` — not yet built).
   */
  minTrust?: number;
}

export const widgets: Record<string, WidgetManifest> = {
  'model-builder': {
    id: 'model-builder',
    title: 'Model Builder',
    icon: '🧠',
    entry: () => import('$lib/components/ModelBuilder.svelte'),
  },
  extensions: {
    id: 'extensions',
    title: 'Extensions',
    icon: '🧩',
    entry: () => import('$lib/panels/ExtensionsPanel.svelte'),
  },
  'self-build': {
    id: 'self-build',
    title: 'Self-Build',
    icon: '🛠',
    entry: () => import('$lib/components/SelfBuildPanel.svelte'),
  },
  survival: {
    id: 'survival',
    title: 'Survival System',
    icon: '🩺',
    entry: () => import('$lib/components/SurvivalPanel.svelte'),
  },
};

export function getWidget(id: string): WidgetManifest | undefined {
  return widgets[id];
}
