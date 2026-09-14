<script lang="ts">
  import { LifeBuoy, Trash2, Undo2, MonitorSmartphone } from 'lucide-svelte';
  import * as api from '../api';
  import type { RecoveryEntry } from '../api';
  import { app } from '../stores/app.svelte';
  import { toast } from '../stores/toast.svelte';
  import { fmtRelative } from '../utils/format';
  import PanelHeader from './PanelHeader.svelte';

  let entries = $state<RecoveryEntry[]>([]);
  let loading = $state(true);
  let selected = $state(0);
  let expandedId = $state<string | null>(null);

  async function refresh(q = app.query) {
    loading = true;
    try {
      entries = await api.getRecoverableTexts(q);
      selected = 0;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (app.panel === 'recover') void refresh(app.query);
  });

  async function recover(entry: RecoveryEntry) {
    await api.recoverText(entry.id);
    toast.success('Restored — paste it anywhere (Ctrl+V)');
  }

  async function remove(entry: RecoveryEntry) {
    await api.deleteRecoveryEntry(entry.id);
    entries = entries.filter((e) => e.id !== entry.id);
  }

  function onKeydown(e: KeyboardEvent) {
    if (app.panel !== 'recover') return;
    const tag = (e.target as HTMLElement)?.tagName;
    if (tag === 'INPUT' && !['ArrowDown', 'ArrowUp', 'Enter'].includes(e.key)) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selected = Math.min(entries.length - 1, selected + 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selected = Math.max(0, selected - 1);
    } else if (e.key === 'Enter' && entries[selected]) {
      e.preventDefault();
      void recover(entries[selected]);
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<PanelHeader icon={LifeBuoy} title="Text Time Machine" badge="FREE" />

<div class="max-h-[380px] overflow-y-auto p-2">
  {#if loading && entries.length === 0}
    <div class="px-4 py-10 text-center text-body text-faint">Scanning the last hour…</div>
  {:else if entries.length === 0}
    <div class="px-4 py-10 text-center">
      <p class="text-body text-muted">Nothing to recover</p>
      <p class="mt-1 text-small text-faint">
        SmartHub snapshots text you type (locally, encrypted) and keeps it for one hour.
      </p>
    </div>
  {:else}
    {#each entries as entry, i (entry.id)}
      <div
        class="group flex w-full items-start gap-3 rounded-xl px-3 py-2.5 transition-colors duration-100
          {i === selected ? 'bg-accent/15' : 'hover:bg-white/5'}"
      >
        <span class="mt-0.5 grid size-9 shrink-0 place-items-center rounded-lg border border-stroke bg-white/5 text-muted">
          <MonitorSmartphone size={15} />
        </span>
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-2">
            <span class="truncate text-small font-medium text-muted">{entry.app}</span>
            <span class="text-[11px] text-faint">·</span>
            <span class="truncate text-[11px] text-faint">{entry.title}</span>
            <span class="ml-auto shrink-0 text-[11px] text-faint">{fmtRelative(entry.createdAt)}</span>
          </div>
          <button
            class="mt-1 block w-full text-left text-body text-ink"
            class:line-clamp-2={expandedId !== entry.id}
            onclick={() => (expandedId = expandedId === entry.id ? null : entry.id)}
            title={expandedId === entry.id ? 'Collapse' : 'Expand'}
          >
            {entry.content}
          </button>
          <div class="mt-1.5 flex items-center gap-2 opacity-0 transition-opacity group-hover:opacity-100">
            <button
              class="inline-flex items-center gap-1.5 rounded-md border border-success/40 bg-success/10 px-2 py-1 text-[11px] font-medium text-success transition-colors hover:bg-success/20"
              onclick={() => recover(entry)}
            >
              <Undo2 size={11} /> Recover
            </button>
            <button
              class="inline-flex items-center gap-1.5 rounded-md border border-stroke bg-surface px-2 py-1 text-[11px] text-muted transition-colors hover:text-danger"
              onclick={() => remove(entry)}
            >
              <Trash2 size={11} /> Forget
            </button>
          </div>
        </div>
      </div>
    {/each}
  {/if}
</div>
