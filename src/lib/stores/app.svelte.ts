// ---------------------------------------------------------------------------
// Central UI store (Svelte 5 runes). Holds which panel is open, the search
// query, and the global command registry that powers the palette.
// ---------------------------------------------------------------------------
import type { Component } from 'svelte';

export type PanelId = 'home' | 'clipboard' | 'recover' | 'modes' | 'ai' | 'license' | 'settings';

export interface Command {
  id: string;
  title: string;
  subtitle?: string;
  keywords: string[];
  icon: string; // resolved by CommandPalette against lucide
  panel?: PanelId;
  run?: () => void | Promise<void>;
  badge?: 'FREE' | 'PRO';
}

function createAppStore() {
  let panel = $state<PanelId>('home');
  let query = $state('');
  let previousPanel: PanelId = 'home';

  return {
    get panel() {
      return panel;
    },
    get query() {
      return query;
    },
    set query(v: string) {
      query = v;
    },
    open(next: PanelId) {
      previousPanel = panel;
      panel = next;
      query = '';
    },
    back() {
      panel = panel === 'home' ? 'home' : previousPanel === panel ? 'home' : 'home';
      query = '';
    },
    reset() {
      panel = 'home';
      query = '';
      previousPanel = 'home';
    }
  };
}

export const app = createAppStore();

/** Simple scoring for fuzzy-ish command matching (prefix > substring). */
export function scoreMatch(q: string, cmd: Command): number {
  if (!q.trim()) return 1;
  const haystacks = [cmd.title, cmd.subtitle ?? '', ...cmd.keywords].map((s) => s.toLowerCase());
  const needle = q.toLowerCase().trim();
  let best = 0;
  for (const h of haystacks) {
    if (!h) continue;
    if (h === needle) best = Math.max(best, 3);
    else if (h.startsWith(needle)) best = Math.max(best, 2);
    else if (h.includes(needle)) best = Math.max(best, 1);
  }
  return best;
}
