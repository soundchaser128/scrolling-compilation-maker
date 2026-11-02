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
  <div class="error">{error}</div>
{:else}
  <div class="progress">
    <progress class="progress" value={pct} max="100" aria-label="Render progress"></progress>
    <span>{status?.state ?? 'queued'} — {pct}%</span>
    {#if status?.state === 'failed'}<div class="error">{status.error}</div>{/if}
    {#if status?.state === 'done' && status.output_path}
      <a href={api.outputUrl(id)} download>Download compilation</a>
    {/if}
  </div>
{/if}

<style>
  .progress { margin: 16px 0; }
  .error { color: #ef5350; }
</style>
