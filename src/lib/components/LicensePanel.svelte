<script lang="ts">
  import { KeyRound, Check, X, Sparkles, Shield, ExternalLink, Loader2 } from 'lucide-svelte';
  import * as api from '../api';
  import { app } from '../stores/app.svelte';
  import { license } from '../stores/license.svelte';
  import { toast } from '../stores/toast.svelte';
  import PanelHeader from './PanelHeader.svelte';

  let key = $state('');
  let busy = $state(false);

  $effect(() => {
    if (app.panel === 'license') void license.refresh();
  });

  const freeFeatures = [
    'Smart Clipboard — 20 items',
    'Text Time Machine — last 1 hour',
    'Search & pin clipboard history',
    'Global hotkey, glass UI'
  ];
  const proFeatures = [
    'Workspace Modes — unlimited profiles',
    'AI File Declutterer — offline, on-device',
    'Creator Quick-Launch (OBS / Discord presets)',
    'Unlimited clipboard history',
    'All future 1.x updates',
    'Lifetime license — pay once'
  ];

  async function activate() {
    if (!key.trim()) return;
    busy = true;
    try {
      const state = await api.activateLicense(key.trim());
      license.set(state);
      toast.success('Pro activated — welcome aboard!');
      key = '';
    } catch (e) {
      toast.error(String(e));
    } finally {
      busy = false;
    }
  }

  async function deactivate() {
    busy = true;
    try {
      const state = await api.deactivateLicense();
      license.set(state);
      toast.info('License removed from this device');
    } finally {
      busy = false;
    }
  }
</script>

<PanelHeader icon={KeyRound} title="SmartHub Pro" />

<div class="max-h-[400px] overflow-y-auto p-4">
  {#if license.isPro}
    <!-- Active license summary -->
    <div class="glass-card rounded-2xl border-accent/30 p-5 text-center">
      <div class="mx-auto grid size-12 place-items-center rounded-2xl bg-success/15 text-success">
        <Shield size={22} />
      </div>
      <h3 class="mt-3 text-title font-semibold">Pro is active</h3>
      <p class="mt-1 font-mono text-small text-muted">{license.state.keyMasked}</p>
      <p class="mt-1 text-[11px] text-faint">
        Lifetime license · validations are cached for 7 days of offline use
      </p>
      <button class="btn-ghost mt-4 !py-1.5 text-small" onclick={deactivate} disabled={busy}>
        Deactivate on this device
      </button>
    </div>
  {:else}
    <!-- Plan comparison -->
    <div class="grid grid-cols-2 gap-3">
      <div class="glass-card rounded-2xl p-4">
        <h3 class="text-body font-semibold text-muted">Free</h3>
        <p class="mt-0.5 text-[22px] font-bold">$0</p>
        <ul class="mt-3 space-y-2">
          {#each freeFeatures as f}
            <li class="flex items-start gap-2 text-small text-muted">
              <Check size={13} class="mt-0.5 shrink-0 text-success" />{f}
            </li>
          {/each}
        </ul>
      </div>
      <div class="relative overflow-hidden rounded-2xl border border-accent/50 bg-accent/10 p-4">
        <div class="absolute -right-8 -top-8 size-24 rounded-full bg-accent/25 blur-2xl"></div>
        <h3 class="flex items-center gap-1.5 text-body font-semibold text-accent-2">
          <Sparkles size={14} /> Pro
        </h3>
        <p class="mt-0.5 text-[22px] font-bold">$19 <span class="text-small font-normal text-muted">lifetime</span></p>
        <ul class="mt-3 space-y-2">
          {#each proFeatures as f}
            <li class="flex items-start gap-2 text-small">
              <Check size={13} class="mt-0.5 shrink-0 text-accent-2" />{f}
            </li>
          {/each}
        </ul>
      </div>
    </div>

    <!-- Activation -->
    <div class="glass-card mt-3 rounded-2xl p-4">
      <label class="mb-2 block text-small font-medium text-muted" for="license-key">
        License key
      </label>
      <div class="flex gap-2">
        <input
          id="license-key"
          class="input flex-1 font-mono"
          placeholder="XXXX-XXXX-XXXX-XXXX"
          bind:value={key}
          spellcheck="false"
          autocomplete="off"
          onkeydown={(e) => e.key === 'Enter' && activate()}
        />
        <button class="btn-accent" onclick={activate} disabled={busy || !key.trim()}>
          {#if busy}<Loader2 size={15} class="animate-spin" />{:else}<Check size={15} />{/if}
          Activate
        </button>
      </div>
      <div class="mt-3 flex items-center justify-between text-[11px] text-faint">
        <span class="flex items-center gap-1.5">
          <Shield size={11} /> Validated once a day · 7-day offline grace
        </span>
        <a
          class="inline-flex items-center gap-1 text-accent-2 hover:underline"
          href="https://lemonsqueezy.com"
          target="_blank"
          rel="noreferrer"
        >
          Buy a license <ExternalLink size={10} />
        </a>
      </div>
    </div>

    <p class="mt-3 flex items-start gap-2 px-1 text-[11px] leading-5 text-faint">
      <X size={11} class="mt-1 shrink-0" />
      Your key is stored encrypted in Windows Credential Manager — never in plain text, never
      synced. Validation only checks the key against the licensing server.
    </p>
  {/if}
</div>
