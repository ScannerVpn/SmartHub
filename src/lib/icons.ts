// ---------------------------------------------------------------------------
// Shared icon component type.
// lucide-svelte 0.x ships Svelte 4-style class components (SvelteComponentTyped);
// they run fine under Svelte 5 (legacy compat), but the *type* differs from
// the runes-era `Component`. One alias keeps all icon records type-safe.
// ---------------------------------------------------------------------------
import type { Icon as LucideIcon } from 'lucide-svelte';

export type IconComponent = typeof LucideIcon;
