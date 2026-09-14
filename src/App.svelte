<script lang="ts">
  import CommandPalette from './lib/components/CommandPalette.svelte';
  import Toasts from './lib/components/Toasts.svelte';
  import { app } from './lib/stores/app.svelte';
  import { license } from './lib/stores/license.svelte';
  import { hideWindow, inTauri } from './lib/api';
  import { Zap } from 'lucide-svelte';

  // Simulated desktop for the browser preview (strings kept single-line so the
  // Svelte style directive parser stays happy).
  const backdropGradient =
    'radial-gradient(1200px 700px at 20% -10%, #2b2350 0%, transparent 55%),' +
    'radial-gradient(1000px 600px at 90% 10%, #14243a 0%, transparent 50%),' +
    'radial-gradient(900px 700px at 50% 110%, #1c1030 0%, transparent 55%), #0a0a0f';
  const dotsPattern = 'radial-gradient(rgba(255,255,255,0.5) 1px, transparent 1px)';

  // In the browser preview the "window" is simulated by this component's
  // `open` flag; inside Tauri the window itself is shown/hidden by the hotkey.
  let open = $state(true);

  $effect(() => {
    void license.refresh();
  });

  function onKeydown(e: KeyboardEvent) {
    if (e.altKey && e.code === 'Space') {
      e.preventDefault();
      open = !open;
      if (open) app.reset();
      return;
    }
    if (e.key === 'Escape') {
      if (app.panel !== 'home') {
        app.back();
        return;
      }
      if (inTauri) void hideWindow();
      else open = false;
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if inTauri}
  <!-- Transparent native window: the palette is the only surface. -->
  <div class="flex h-full w-full items-start justify-center pt-[16vh]">
    <div class="w-full max-w-[720px] animate-palette-in px-4">
      <CommandPalette />
    </div>
  </div>
{:else}
  <!-- Browser preview: simulated desktop so the glass effect is visible. -->
  <div class="relative h-full w-full overflow-hidden" style:background={backdropGradient}>
    <div
      class="absolute inset-0 opacity-[0.13]"
      style:background-image={dotsPattern}
      style:background-size="26px 26px"
    ></div>

    {#if open}
      <div class="relative flex h-full w-full items-start justify-center pt-[14vh]">
        <div class="w-full max-w-[720px] animate-palette-in px-4">
          <CommandPalette />
        </div>
      </div>
    {:else}
      <button
        class="glass absolute left-1/2 top-1/2 flex -translate-x-1/2 -translate-y-1/2 items-center gap-2.5 rounded-2xl px-6 py-4 text-body text-ink transition-transform hover:scale-105"
        onclick={() => { open = true; app.reset(); }}
      >
        <span class="grid size-9 place-items-center rounded-xl bg-accent/20 text-accent-2">
          <Zap size={17} />
        </span>
        SmartHub is running — press
        <span class="kbd">Alt</span> + <span class="kbd">Space</span>
      </button>
    {/if}

    <div class="absolute bottom-4 left-1/2 -translate-x-1/2 text-[11px] text-white/35">
      Browser preview — inside the app, this window floats over your desktop
    </div>
  </div>
{/if}

<Toasts />
