<script lang="ts">
  // Shared header for module panels: back chevron, icon, title, actions slot.
  import { ChevronLeft } from 'lucide-svelte';
  import { app } from '../stores/app.svelte';
  import type { Snippet } from 'svelte';
  import type { IconComponent } from '../icons';

  let {
    icon: Icon,
    title,
    badge,
    actions
  }: {
    icon: IconComponent;
    title: string;
    badge?: 'FREE' | 'PRO';
    actions?: Snippet;
  } = $props();
</script>

<div class="flex items-center gap-2.5 border-b border-stroke px-4 py-2.5">
  <button
    class="grid size-7 place-items-center rounded-lg text-muted transition-colors hover:bg-white/10 hover:text-ink"
    onclick={() => app.back()}
    title="Back (Esc)"
    aria-label="Back"
  >
    <ChevronLeft size={16} />
  </button>
  <div class="grid size-7 place-items-center rounded-lg bg-accent/15 text-accent-2">
    <Icon size={15} />
  </div>
  <h2 class="text-body font-medium">{title}</h2>
  {#if badge}
    <span
      class="chip {badge === 'PRO'
        ? 'border-accent/40 bg-accent/15 text-accent-2'
        : 'border-success/30 bg-success/10 text-success'}"
    >
      {badge}
    </span>
  {/if}
  <div class="ml-auto flex items-center gap-1.5">
    {#if actions}
      {@render actions()}
    {/if}
  </div>
</div>
