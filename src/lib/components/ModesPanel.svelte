<script lang="ts">
  import {
    Layers, Moon, Radio, Code, Rocket, Gamepad2, Play, Plus, Volume2, BellOff,
    Image as ImageIcon, XCircle, TerminalSquare, Hourglass
  } from 'lucide-svelte';
  import type { IconComponent } from '../icons';
  import * as api from '../api';
  import type { ProfileActionKind, WorkspaceProfile } from '../api';
  import { app } from '../stores/app.svelte';
  import { license } from '../stores/license.svelte';
  import { toast } from '../stores/toast.svelte';
  import PanelHeader from './PanelHeader.svelte';
  import ProLock from './ProLock.svelte';

  let profiles = $state<WorkspaceProfile[]>([]);
  let activeId = $state<string | null>(null);
  let applyingId = $state<string | null>(null);
  let loaded = $state(false);

  const profileIcons: Record<string, IconComponent> = {
    moon: Moon,
    radio: Radio,
    code: Code,
    rocket: Rocket,
    gamepad: Gamepad2,
    layers: Layers
  };

  const actionMeta: Record<ProfileActionKind, { label: string; icon: IconComponent }> = {
    launch_app: { label: 'Launch', icon: Play },
    kill_app: { label: 'Quit', icon: XCircle },
    set_wallpaper: { label: 'Wallpaper', icon: ImageIcon },
    set_volume: { label: 'Volume', icon: Volume2 },
    mute: { label: 'Mute', icon: Volume2 },
    do_not_disturb: { label: 'DND', icon: BellOff },
    run_command: { label: 'Command', icon: TerminalSquare }
  };

  async function refresh() {
    const res = await api.getWorkspaceProfiles();
    profiles = res.profiles;
    activeId = res.activeId;
    loaded = true;
  }

  $effect(() => {
    if (app.panel === 'modes') void refresh();
  });

  async function apply(profile: WorkspaceProfile) {
    applyingId = profile.id;
    try {
      await api.applyWorkspaceProfile(profile.id);
      activeId = profile.id;
      toast.success(`"${profile.name}" mode applied`);
    } catch (e) {
      if (String(e).includes('pro_required')) toast.error('Workspace Modes is a Pro feature');
      else toast.error(`Failed: ${e}`);
    } finally {
      applyingId = null;
    }
  }
</script>

<PanelHeader icon={Layers} title="Workspace Modes" badge="PRO">
  {#snippet actions()}
    {#if license.isPro}
      <button class="btn-ghost !px-2.5 !py-1.5 text-small" onclick={() => toast.info('Profile editor: pick actions, save. Full editor in next build.')}>
        <Plus size={14} /> New profile
      </button>
    {/if}
  {/snippet}
</PanelHeader>

{#if !license.isPro}
  <ProLock module="Workspace Modes" />
{:else if !loaded}
  <div class="px-4 py-10 text-center text-body text-faint">Loading profiles…</div>
{:else}
  <div class="grid grid-cols-2 gap-3 p-3">
    {#each profiles as profile (profile.id)}
      {@const Icon = profileIcons[profile.icon] ?? Layers}
      {@const isActive = activeId === profile.id}
      <button
        class="glass-card group relative rounded-2xl p-4 text-left transition-all duration-150 hover:-translate-y-0.5 hover:border-accent/40
          {isActive ? 'border-accent/60 bg-accent/10' : ''}"
        onclick={() => apply(profile)}
        disabled={applyingId !== null}
      >
        {#if isActive}
          <span class="absolute right-3 top-3 size-2 rounded-full bg-success shadow-[0_0_10px_rgba(16,185,129,0.8)]"></span>
        {/if}
        <div class="grid size-10 place-items-center rounded-xl bg-accent/15 text-accent-2">
          {#if applyingId === profile.id}
            <Hourglass size={18} class="animate-pulse" />
          {:else}
            <Icon size={18} />
          {/if}
        </div>
        <h3 class="mt-3 text-body font-semibold">{profile.name}</h3>
        <div class="mt-2 flex flex-wrap gap-1">
          {#each profile.actions.slice(0, 4) as action}
            {@const meta = actionMeta[action.kind]}
            <span class="chip"><meta.icon size={10} />{action.value.length > 18 ? action.value.slice(0, 16) + '…' : action.value}</span>
          {/each}
          {#if profile.actions.length > 4}
            <span class="chip">+{profile.actions.length - 4}</span>
          {/if}
        </div>
        <p class="mt-3 text-[11px] text-faint">{profile.actions.length} actions · click to apply</p>
      </button>
    {/each}
  </div>
{/if}
