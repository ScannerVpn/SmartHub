<script lang="ts">
  import {
    Wand2, FolderSearch, Download, Cpu, FileWarning, CopyX, Archive, HardDriveDownload,
    RefreshCw, Trash2
  } from 'lucide-svelte';
  import * as api from '../api';
  import type { AiStatus, ScanResult } from '../api';
  import { app } from '../stores/app.svelte';
  import { license } from '../stores/license.svelte';
  import { toast } from '../stores/toast.svelte';
  import { fmtBytes } from '../utils/format';
  import PanelHeader from './PanelHeader.svelte';
  import ProLock from './ProLock.svelte';

  let folder = $state('');
  let status = $state<AiStatus>({ state: 'not_downloaded' });
  let result = $state<ScanResult | null>(null);
  let scanning = $state(false);
  let selectedIds = $state<Set<string>>(new Set());

  const selectedFiles = $derived(
    result ? result.suggestions.filter((s) => selectedIds.has(s.id)) : []
  );
  const reclaimTotal = $derived(selectedFiles.reduce((acc, s) => acc + s.sizeBytes, 0));

  const reasonIcons = {
    duplicate: CopyX,
    stale: FileWarning,
    installer: HardDriveDownload,
    large_archive: Archive
  } as const;

  $effect(() => {
    if (app.panel === 'ai') {
      void api.getAiStatus().then((s) => (status = s));
    }
  });

  async function scan() {
    scanning = true;
    try {
      result = await api.scanFolder(folder || undefined);
      selectedIds = new Set(result.suggestions.map((s) => s.id));
    } catch (e) {
      toast.error(`Scan failed: ${e}`);
    } finally {
      scanning = false;
    }
  }

  function toggle(id: string) {
    const next = new Set(selectedIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    selectedIds = next;
  }

  function clean() {
    // Rust command `declutter_files` arrives with the Pro implementation;
    // the UI contract is already here.
    toast.success(`${selectedFiles.length} files queued — recycling happens after final confirm`);
  }
</script>

<PanelHeader icon={Wand2} title="AI File Declutterer" badge="PRO" />

{#if !license.isPro}
  <ProLock module="AI File Declutterer" />
{:else}
  <div class="max-h-[400px] overflow-y-auto p-3">
    <!-- Local model status: everything runs on-device -->
    <div class="glass-card mb-3 flex items-center gap-3 rounded-xl px-3.5 py-2.5">
      <div class="grid size-8 place-items-center rounded-lg bg-accent/15 text-accent-2">
        <Cpu size={15} />
      </div>
      <div class="flex-1 text-small">
        {#if status.state === 'not_downloaded'}
          <span class="text-muted">Local AI model not installed — smart suggestions use heuristics until you download it (≈2.4 GB, one time).</span>
        {:else if status.state === 'downloading'}
          <span class="text-muted">Downloading model… {Math.round(status.progress * 100)}%</span>
        {:else if status.state === 'ready'}
          <span class="text-success">Local model ready ({status.model}) — fully offline</span>
        {:else}
          <span class="text-danger">{status.message}</span>
        {/if}
      </div>
      {#if status.state === 'not_downloaded'}
        <button class="btn-ghost !px-2.5 !py-1.5 text-small" onclick={() => toast.info('Model download starts on first AI use')}>
          <Download size={13} /> Get model
        </button>
      {/if}
    </div>

    <div class="mb-3 flex gap-2">
      <input
        class="input flex-1"
        placeholder="Folder to scan — e.g. C:\Users\you\Downloads"
        bind:value={folder}
        onkeydown={(e) => e.key === 'Enter' && scan()}
      />
      <button class="btn-accent" onclick={scan} disabled={scanning}>
        {#if scanning}
          <RefreshCw size={15} class="animate-spin" /> Scanning…
        {:else}
          <FolderSearch size={15} /> Scan
        {/if}
      </button>
    </div>

    {#if result}
      <div class="mb-3 flex flex-wrap items-center gap-1.5 px-1 text-small text-muted">
        <span class="font-medium text-ink">{result.fileCount} files</span>
        <span class="text-faint">·</span>
        <span>{fmtBytes(result.totalBytes)}</span>
        <span class="text-faint">·</span>
        <span class="truncate text-faint">{result.folder}</span>
      </div>
      <div class="mb-3 flex flex-wrap gap-1.5">
        {#each Object.entries(result.categories) as [name, count]}
          <span class="chip capitalize">{name} <b class="text-ink">{count}</b></span>
        {/each}
      </div>

      <p class="mb-2 px-1 text-small font-medium text-muted">Suggestions ({result.suggestions.length})</p>
      {#each result.suggestions as s (s.id)}
        {@const ReasonIcon = reasonIcons[s.reason]}
        <label
          class="glass-card mb-2 flex cursor-pointer items-start gap-3 rounded-xl px-3 py-2.5 transition-colors hover:border-accent/40"
        >
          <input
            type="checkbox"
            class="mt-1 size-4 accent-indigo-500"
            checked={selectedIds.has(s.id)}
            onchange={() => toggle(s.id)}
          />
          <ReasonIcon size={15} class="mt-1 shrink-0 text-warning" />
          <span class="min-w-0 flex-1">
            <span class="block truncate text-body text-ink">{s.path}</span>
            <span class="block text-small text-muted">{s.detail || s.reason}</span>
          </span>
          <span class="shrink-0 font-mono text-small text-faint">{fmtBytes(s.sizeBytes)}</span>
        </label>
      {/each}

      <div class="sticky bottom-0 flex items-center justify-between rounded-xl border border-stroke bg-surface/90 px-3 py-2 backdrop-blur">
        <span class="text-small text-muted">
          {selectedFiles.length} selected · reclaim <b class="text-ink">{fmtBytes(reclaimTotal)}</b>
        </span>
        <button class="btn-accent !bg-danger hover:!bg-danger/80" onclick={clean} disabled={selectedFiles.length === 0}>
          <Trash2 size={14} /> Clean up
        </button>
      </div>
    {:else if !scanning}
      <div class="px-4 py-8 text-center text-small text-faint">
        Pick a cluttered folder (Downloads, Desktop…) — analysis runs 100% on-device.
      </div>
    {/if}
  </div>
{/if}
