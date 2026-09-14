<script lang="ts">
  import { Settings, Keyboard, Save, Loader2 } from 'lucide-svelte';
  import * as api from '../api';
  import type { AppSettings } from '../api';
  import { app } from '../stores/app.svelte';
  import { toast } from '../stores/toast.svelte';
  import PanelHeader from './PanelHeader.svelte';

  let settings = $state<AppSettings | null>(null);
  let dirty = $state(false);
  let saving = $state(false);
  let recording = $state(false);

  $effect(() => {
    if (app.panel === 'settings' && !settings) {
      void api.getSettings().then((s) => (settings = s));
    }
  });

  function update<K extends keyof AppSettings>(k: K, v: AppSettings[K]) {
    if (!settings) return;
    settings = { ...settings, [k]: v };
    dirty = true;
  }

  async function save() {
    if (!settings) return;
    saving = true;
    try {
      settings = await api.saveSettings(settings);
      dirty = false;
      toast.success('Settings saved');
    } catch (e) {
      toast.error(String(e));
    } finally {
      saving = false;
    }
  }

  /** Capture the next key combo as the new global hotkey (Tauri syntax). */
  function onRecordKeydown(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    if (e.key === 'Escape') {
      recording = false;
      return;
    }
    if (['Alt', 'Control', 'Shift', 'Meta'].includes(e.key)) return;
    const parts: string[] = [];
    if (e.ctrlKey) parts.push('Ctrl');
    if (e.altKey) parts.push('Alt');
    if (e.shiftKey) parts.push('Shift');
    if (e.metaKey) parts.push('Super');
    let k = e.key;
    if (k === ' ') k = 'Space';
    else if (k.length === 1) k = k.toUpperCase();
    parts.push(k);
    update('hotkey', parts.join('+'));
    recording = false;
  }
</script>

<svelte:window onkeydown={onRecordKeydown} />

<PanelHeader icon={Settings} title="Settings">
  {#snippet actions()}
    <button class="btn-accent !px-2.5 !py-1.5 text-small" onclick={save} disabled={!dirty || saving}>
      {#if saving}<Loader2 size={13} class="animate-spin" />{:else}<Save size={13} />{/if}
      Save
    </button>
  {/snippet}
</PanelHeader>

{#if settings}
  <div class="max-h-[400px] space-y-1 overflow-y-auto p-3">
    <!-- Hotkey -->
    <div class="glass-card flex items-center gap-3 rounded-xl px-3.5 py-3">
      <div class="grid size-8 place-items-center rounded-lg bg-accent/15 text-accent-2">
        <Keyboard size={15} />
      </div>
      <div class="flex-1">
        <p class="text-body">Global hotkey</p>
        <p class="text-small text-faint">Opens SmartHub from anywhere</p>
      </div>
      <button
        class="rounded-lg border px-3 py-1.5 font-mono text-small transition-colors
          {recording ? 'border-accent bg-accent/15 text-accent-2' : 'border-stroke bg-white/5 text-ink hover:border-accent/50'}"
        onclick={() => (recording = !recording)}
      >
        {recording ? 'Press a combo…' : settings.hotkey}
      </button>
    </div>

    <!-- Toggles -->
    {#each [
      { key: 'launchAtLogin', label: 'Launch at login', hint: 'Start SmartHub minimized with Windows' },
      { key: 'clipboardMonitoring', label: 'Clipboard monitoring', hint: 'Watch and store copied items locally' },
      { key: 'textCaptureEnabled', label: 'Text Time Machine', hint: 'Snapshot typed text for one hour (encrypted)' }
    ] as row}
      <div class="glass-card flex items-center gap-3 rounded-xl px-3.5 py-3">
        <div class="flex-1">
          <p class="text-body">{row.label}</p>
          <p class="text-small text-faint">{row.hint}</p>
        </div>
        <button
          role="switch"
          aria-checked={settings[row.key as keyof AppSettings] as boolean}
          aria-label={row.label}
          class="relative h-6 w-11 rounded-full transition-colors
            {settings[row.key as keyof AppSettings] ? 'bg-accent' : 'bg-white/10'}"
          onclick={() => update(row.key as keyof AppSettings, !settings?.[row.key as keyof AppSettings] as never)}
        >
          <span
            class="absolute top-0.5 size-5 rounded-full bg-white transition-all
              {settings[row.key as keyof AppSettings] ? 'left-[22px]' : 'left-0.5'}"
          ></span>
        </button>
      </div>
    {/each}

    <!-- Clipboard depth -->
    <div class="glass-card rounded-xl px-3.5 py-3">
      <div class="flex items-center justify-between">
        <div>
          <p class="text-body">Clipboard history depth</p>
          <p class="text-small text-faint">Free tier keeps the latest 20 items</p>
        </div>
        <span class="font-mono text-small text-accent-2">{settings.maxClipboardItems}</span>
      </div>
      <input
        type="range"
        min="20"
        max="500"
        step="10"
        class="mt-3 w-full accent-indigo-500"
        value={settings.maxClipboardItems}
        oninput={(e) => update('maxClipboardItems', Number((e.target as HTMLInputElement).value))}
      />
    </div>

    <p class="px-2 pt-2 text-center text-[11px] text-faint">
      SmartHub 0.1.0 · Data never leaves this device · Zero telemetry
    </p>
  </div>
{/if}
