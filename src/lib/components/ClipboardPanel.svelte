<script lang="ts">
  import {
    Clipboard, Link2, Code2, Type, Image as ImageIcon, Pin, PinOff, Trash2, RefreshCw, Palette as PaletteIcon
  } from 'lucide-svelte';
  import * as api from '../api';
  import type { ClipboardItem, ClipboardKind } from '../api';
  import { app } from '../stores/app.svelte';
  import { toast } from '../stores/toast.svelte';
  import { fmtRelative } from '../utils/format';
  import type { IconComponent } from '../icons';
  import PanelHeader from './PanelHeader.svelte';

  let items = $state<ClipboardItem[]>([]);
  let loading = $state(true);
  let selected = $state(0);

  const kindIcons: Record<ClipboardKind, IconComponent> = {
    text: Type,
    link: Link2,
    code: Code2,
    image: ImageIcon,
    color: PaletteIcon
  };

  async function refresh(q = app.query) {
    loading = true;
    try {
      items = await api.getClipboardHistory(q);
      selected = 0;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (app.panel === 'clipboard') void refresh(app.query);
  });

  async function paste(item: ClipboardItem) {
    await api.copyToClipboard(item.id);
    toast.success('Copied to clipboard');
    if (api.inTauri) await api.hideWindow();
  }

  async function togglePin(item: ClipboardItem) {
    const updated = await api.togglePin(item.id, !item.pinned);
    if (updated) items = items.map((c) => (c.id === updated.id ? updated : c));
  }

  async function remove(item: ClipboardItem) {
    await api.deleteClipboardItem(item.id);
    items = items.filter((c) => c.id !== item.id);
    toast.info('Removed from history');
  }

  function onKeydown(e: KeyboardEvent) {
    if (app.panel !== 'clipboard') return;
    const tag = (e.target as HTMLElement)?.tagName;
    if (tag === 'INPUT' && !['ArrowDown', 'ArrowUp', 'Enter'].includes(e.key)) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selected = Math.min(items.length - 1, selected + 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selected = Math.max(0, selected - 1);
    } else if (e.key === 'Enter' && items[selected]) {
      e.preventDefault();
      void paste(items[selected]);
    } else if (e.key.toLowerCase() === 'p' && tag !== 'INPUT' && items[selected]) {
      void togglePin(items[selected]);
    } else if (e.key === 'Delete' && items[selected]) {
      void remove(items[selected]);
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<PanelHeader icon={Clipboard} title="Smart Clipboard" badge="FREE">
  {#snippet actions()}
    <button
      class="grid size-7 place-items-center rounded-lg text-muted transition-colors hover:bg-white/10 hover:text-ink
        {loading ? 'animate-spin' : ''}"
      onclick={() => refresh()}
      title="Refresh"
    >
      <RefreshCw size={14} />
    </button>
  {/snippet}
</PanelHeader>

<div class="max-h-[380px] overflow-y-auto p-2">
  {#if loading && items.length === 0}
    <div class="px-4 py-10 text-center text-body text-faint">Loading history…</div>
  {:else if items.length === 0}
    <div class="px-4 py-10 text-center">
      <p class="text-body text-muted">Nothing here yet</p>
      <p class="mt-1 text-small text-faint">Copy something — SmartHub keeps your last 20 items, encrypted, on-device.</p>
    </div>
  {:else}
    {#each items as item, i (item.id)}
      {@const Icon = kindIcons[item.kind]}
      <div
        class="group relative flex w-full items-start gap-3 rounded-xl px-3 py-2.5 text-left transition-colors duration-100
          {i === selected ? 'bg-accent/15' : 'hover:bg-white/5'}"
      >
        <button class="flex flex-1 items-start gap-3 text-left" onclick={() => paste(item)}>
          {#if item.kind === 'image'}
            <img src={item.content} alt="" class="mt-0.5 h-9 w-[72px] rounded-md border border-stroke object-cover" />
          {:else if item.kind === 'color'}
            <span
              class="mt-0.5 grid size-9 place-items-center rounded-lg border border-stroke"
              style:background={item.content}
            >
              <PaletteIcon size={14} class="text-white/90" />
            </span>
          {:else}
            <span class="mt-0.5 grid size-9 shrink-0 place-items-center rounded-lg border border-stroke bg-white/5 text-muted">
              <Icon size={15} />
            </span>
          {/if}

          <span class="min-w-0 flex-1">
            {#if item.linkTitle}
              <span class="block truncate text-body text-ink">{item.linkTitle}</span>
              <span class="block truncate text-small text-faint">{item.preview}</span>
            {:else if item.kind === 'code'}
              <span class="block truncate font-mono text-small text-ink">{item.preview}</span>
            {:else}
              <span class="block truncate text-body text-ink">{item.preview || item.content}</span>
            {/if}
            <span class="mt-1 flex items-center gap-2 text-[11px] text-faint">
              <span>{fmtRelative(item.createdAt)}</span>
              {#if item.useCount > 1}<span>used ×{item.useCount}</span>{/if}
              {#if item.pinned}
                <span class="inline-flex items-center gap-1 text-accent-2"><Pin size={10} /> pinned</span>
              {/if}
            </span>
          </span>
        </button>

        <span class="absolute right-2 top-2 hidden gap-1 group-hover:flex">
          <button
            class="grid size-7 place-items-center rounded-md border border-stroke bg-surface text-muted transition-colors hover:text-ink"
            onclick={() => togglePin(item)}
            title={item.pinned ? 'Unpin (P)' : 'Pin (P)'}
          >
            {#if item.pinned}<PinOff size={13} />{:else}<Pin size={13} />{/if}
          </button>
          <button
            class="grid size-7 place-items-center rounded-md border border-stroke bg-surface text-muted transition-colors hover:text-danger"
            onclick={() => remove(item)}
            title="Delete (Del)"
          >
            <Trash2 size={13} />
          </button>
        </span>
      </div>
    {/each}
  {/if}
</div>
