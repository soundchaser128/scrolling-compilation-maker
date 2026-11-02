<script lang="ts">
  import { api } from '$lib/api';
  import type { RenderStatus } from '$lib/types';

  let { id }: { id: string } = $props();
  let status = $state<RenderStatus | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    const timer = setInterval(async () => {
      try {
        status = await api.status(id);
        if (status && (status.state === 'done' || status.state === 'failed')) clearInterval(timer);
      } catch (e) {
        error = String(e);
        clearInterval(timer);
      }
    }, 1000);
    return () => clearInterval(timer);
  });

  const pct = $derived(Math.round((status?.progress ?? 0) * 100));
</script>

{#if error}
  <div class="alert alert-error mt-6" role="alert">{error}</div>
{:else}
  <section class="card card-border mt-6 bg-base-100"><div class="card-body"><div class="flex flex-wrap items-center justify-between gap-3"><div><div class="eyebrow">RENDER STATUS</div><h2 class="mt-2 text-xl font-bold capitalize">{status?.state ?? 'queued'}</h2></div><span class="text-2xl font-bold">{pct}%</span></div><progress class="progress progress-neutral mt-4 w-full" value={pct} max="100" aria-label="Render progress"></progress>{#if status?.state === 'failed'}<div class="alert alert-error mt-3" role="alert">{status.error}</div>{/if}{#if status?.state === 'done' && status.output_path}<a class="btn btn-neutral mt-4 self-start" href={api.outputUrl(id)} download>Download compilation ↓</a>{/if}</div></section>
{/if}
