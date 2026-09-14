<script lang="ts">
  import { toast } from '../stores/toast.svelte';
  import { Check, AlertTriangle, Info } from 'lucide-svelte';

  const icons = { success: Check, error: AlertTriangle, info: Info };
</script>

<div class="pointer-events-none fixed bottom-5 left-1/2 z-50 flex -translate-x-1/2 flex-col items-center gap-2">
  {#each toast.list as t (t.id)}
    {@const Icon = icons[t.kind]}
    <div
      class="glass pointer-events-auto flex items-center gap-2.5 rounded-xl px-4 py-2.5 text-body animate-spring-in"
      role="status"
    >
      <span
        class="grid size-5 place-items-center rounded-full
        {t.kind === 'success' ? 'bg-success/20 text-success' : ''}
        {t.kind === 'error' ? 'bg-danger/20 text-danger' : ''}
        {t.kind === 'info' ? 'bg-accent/20 text-accent-2' : ''}"
      >
        <Icon size={13} strokeWidth={2.5} />
      </span>
      {t.message}
    </div>
  {/each}
</div>
