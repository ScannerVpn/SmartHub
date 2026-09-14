<script lang="ts">
  import { Lock, Sparkles } from 'lucide-svelte';
  import { app } from '../stores/app.svelte';

  // Shown over Pro panels when the user is on the Free tier.
  let { module }: { module: string } = $props();
</script>

<div class="flex h-full flex-col items-center justify-center gap-4 px-8 py-14 text-center">
  <div class="grid size-14 place-items-center rounded-2xl bg-accent/15 text-accent-2">
    <Lock size={24} />
  </div>
  <div>
    <h3 class="text-title font-semibold">{module} — ویژگی Pro</h3>
    <p class="mx-auto mt-2 max-w-sm text-body text-muted">
      این ماژول بخشی از SmartHub Pro است. با لایسنس مادام‌العمر ۱۹ دلاری فعالش کن — بدون اشتراک، بدون قفل زمانی.
    </p>
  </div>
  <button class="btn-accent" onclick={() => app.open('license')}>
    <Sparkles size={16} />
    Upgrade to Pro — $19 lifetime
  </button>
  <span class="text-small text-faint">30-day money-back · works offline for 7 days after validation</span>
</div>
