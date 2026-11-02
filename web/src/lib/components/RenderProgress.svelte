<script lang="ts">
  import { api } from '#lib/api.js';
  import type { RenderStatus } from '#lib/types.js';

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
  <div class="mt-6 alert alert-error" role="alert">{error}</div>
{:else}
  <section class="card mt-6 bg-base-100 card-border">
    <div class="card-body">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div>
          <div class="eyebrow">RENDER STATUS</div>
          <h2 class="mt-2 text-xl font-bold capitalize">{status?.state ?? 'queued'}</h2>
        </div>
        <span class="text-2xl font-bold">{pct}%</span>
      </div>
      <progress
        class="progress mt-4 w-full progress-neutral"
        value={pct}
        max="100"
        aria-label="Render progress"
      ></progress>{#if status?.state === 'failed'}<div class="mt-3 alert alert-error" role="alert">
          {status.error}
        </div>{/if}{#if status?.state === 'done' && status.output_path}<a
          class="btn mt-4 self-start btn-neutral"
          href={api.outputUrl(id)}
          download>Download compilation ↓</a
        >{/if}
    </div>
  </section>
{/if}
