<script lang="ts">
  import {
    Search, Clipboard, LifeBuoy, Layers, Wand2, KeyRound, Settings, CornerDownLeft,
    ChevronRight
  } from 'lucide-svelte';
  import type { IconComponent } from '../icons';
  import { app, scoreMatch, type Command } from '../stores/app.svelte';
  import { license } from '../stores/license.svelte';
  import ClipboardPanel from './ClipboardPanel.svelte';
  import RecoveryPanel from './RecoveryPanel.svelte';
  import ModesPanel from './ModesPanel.svelte';
  import AICleaner from './AICleaner.svelte';
  import LicensePanel from './LicensePanel.svelte';
  import SettingsPanel from './SettingsPanel.svelte';

  let inputEl: HTMLInputElement | undefined = $state();
  let selected = $state(0);

  const commandIcons: Record<string, IconComponent> = {
    clipboard: Clipboard,
    recover: LifeBuoy,
    modes: Layers,
    ai: Wand2,
    license: KeyRound,
    settings: Settings
  };

  const commands: Command[] = [
    {
      id: 'clipboard',
      title: 'Smart Clipboard',
      subtitle: 'History, links with titles, pinned snippets',
      keywords: ['copy', 'paste', 'history', 'کلیپ‌بورد'],
      icon: 'clipboard',
      panel: 'clipboard',
      badge: 'FREE'
    },
    {
      id: 'recover',
      title: 'Recover lost text',
      subtitle: 'Text Time Machine — rescue what you typed',
      keywords: ['recover', 'rescue', 'time machine', 'typed', 'نجات', 'متن'],
      icon: 'recover',
      panel: 'recover',
      badge: 'FREE'
    },
    {
      id: 'modes',
      title: 'Workspace modes',
      subtitle: 'Focus / Streaming / Coding — one-click system state',
      keywords: ['focus', 'gaming', 'streaming', 'coding', 'profile', 'mode'],
      icon: 'modes',
      panel: 'modes',
      badge: 'PRO'
    },
    {
      id: 'ai',
      title: 'AI File Declutterer',
      subtitle: 'Clean Downloads & Desktop — fully on-device',
      keywords: ['clean', 'declutter', 'downloads', 'duplicates', 'آرشیو'],
      icon: 'ai',
      panel: 'ai',
      badge: 'PRO'
    },
    {
      id: 'license',
      title: 'SmartHub Pro — $19 lifetime',
      subtitle: 'Activate or manage your license',
      keywords: ['pro', 'activate', 'buy', 'license', 'key', 'لایسنس'],
      icon: 'license',
      panel: 'license'
    },
    {
      id: 'settings',
      title: 'Settings',
      subtitle: 'Hotkey, monitoring, privacy',
      keywords: ['preferences', 'hotkey', 'config', 'تنظیمات'],
      icon: 'settings',
      panel: 'settings'
    }
  ];

  const matches = $derived(
    commands
      .map((c) => ({ cmd: c, score: scoreMatch(app.query, c) }))
      .filter((m) => m.score > 0)
      .sort((a, b) => b.score - a.score)
      .map((m) => m.cmd)
  );

  $effect(() => {
    // Panel switches reset the selection cursor; the search stays focused.
    app.panel;
    selected = 0;
    inputEl?.focus();
  });

  function run(cmd: Command) {
    if (cmd.panel) app.open(cmd.panel);
    else void cmd.run?.();
  }

  function onInputKeydown(e: KeyboardEvent) {
    if (app.panel !== 'home') return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selected = Math.min(matches.length - 1, selected + 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selected = Math.max(0, selected - 1);
    } else if (e.key === 'Enter' && matches[selected]) {
      e.preventDefault();
      run(matches[selected]);
    } else if (e.key === 'Tab' && matches[selected]) {
      e.preventDefault();
      run(matches[selected]);
    }
  }

  const placeholder = $derived(
    app.panel === 'home'
      ? 'Type a command or search…'
      : app.panel === 'clipboard'
        ? 'Search clipboard history…'
        : app.panel === 'recover'
          ? 'Search rescued text…'
          : 'Type to search…'
  );
</script>

<div class="glass overflow-hidden rounded-2xl">
  <!-- Search bar -->
  <div class="flex items-center gap-3 border-b border-stroke px-4">
    <Search size={17} class="shrink-0 text-faint" />
    <input
      bind:this={inputEl}
      bind:value={app.query}
      onkeydown={onInputKeydown}
      class="h-[52px] w-full bg-transparent text-title text-ink outline-none placeholder:text-faint"
      {placeholder}
      spellcheck="false"
      autocomplete="off"
    />
    {#if app.panel !== 'home'}
      <button class="kbd" onclick={() => app.back()}>esc · back</button>
    {/if}
  </div>

  <!-- Body -->
  {#if app.panel === 'home'}
    <div class="max-h-[380px] overflow-y-auto p-2">
      {#if matches.length === 0}
        <p class="px-4 py-10 text-center text-body text-faint">No commands match “{app.query}”</p>
      {:else}
        {#each matches as cmd, i (cmd.id)}
          {@const Icon = commandIcons[cmd.icon]}
          <button
            class="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left transition-colors duration-100
              {i === selected ? 'bg-accent/15' : 'hover:bg-white/5'}"
            onclick={() => run(cmd)}
            onmouseenter={() => (selected = i)}
          >
            <span class="grid size-9 shrink-0 place-items-center rounded-lg border border-stroke bg-white/5 text-accent-2">
              <Icon size={15} />
            </span>
            <span class="min-w-0 flex-1">
              <span class="flex items-center gap-2 text-body font-medium">
                {cmd.title}
                {#if cmd.badge}
                  <span
                    class="rounded-full px-1.5 py-px text-[9px] font-bold tracking-wider
                      {cmd.badge === 'PRO' ? 'bg-accent/20 text-accent-2' : 'bg-success/15 text-success'}"
                  >
                    {cmd.badge}
                  </span>
                {/if}
              </span>
              {#if cmd.subtitle}
                <span class="block truncate text-small text-faint">{cmd.subtitle}</span>
              {/if}
            </span>
            <ChevronRight size={15} class="shrink-0 text-faint" />
          </button>
        {/each}
      {/if}
    </div>
  {:else if app.panel === 'clipboard'}
    <ClipboardPanel />
  {:else if app.panel === 'recover'}
    <RecoveryPanel />
  {:else if app.panel === 'modes'}
    <ModesPanel />
  {:else if app.panel === 'ai'}
    <AICleaner />
  {:else if app.panel === 'license'}
    <LicensePanel />
  {:else if app.panel === 'settings'}
    <SettingsPanel />
  {/if}

  <!-- Footer -->
  <div class="flex items-center gap-4 border-t border-stroke px-4 py-2.5 text-[11px] text-faint">
    <span class="flex items-center gap-1.5"><span class="kbd"><CornerDownLeft size={10} /></span> open</span>
    <span class="flex items-center gap-1.5"><span class="kbd">↑ ↓</span> navigate</span>
    <span class="flex items-center gap-1.5"><span class="kbd">esc</span> {app.panel === 'home' ? 'close' : 'back'}</span>
    <span class="ml-auto flex items-center gap-1.5">
      <span class="size-1.5 rounded-full {license.isPro ? 'bg-accent-2' : 'bg-success'}"></span>
      SmartHub · {license.loaded ? (license.isPro ? 'Pro' : 'Free') : '…'}
    </span>
  </div>
</div>
