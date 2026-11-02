<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { onMount } from 'svelte';
  import { api } from '#lib/api.js';
  import type { ClipSelection, RenderRequest } from '#lib/types.js';
  import { estimateVisibility, excerptRange } from '#lib/timing.js';
  import RenderForm from '#lib/components/RenderForm.svelte';
  import RenderProgress from '#lib/components/RenderProgress.svelte';
  import { workflow, saveWorkflow } from '#lib/workflow.svelte.js';

  let loading = $state(false);
  let error = $state<string | null>(null);
  onMount(() => {
    if (workflow.clips === null) void goto(resolve('/'), { replaceState: true });
    else if (workflow.selected.length < 2) void goto(resolve('/clips'), { replaceState: true });
  });

  async function onRender(opts: Record<string, unknown>) {
    if (loading) return;
    const chosen = (workflow.clips ?? []).filter((c) => workflow.selected.includes(c.id));
    const visibility = estimateVisibility(chosen, workflow.timing);
    const selections: ClipSelection[] = chosen.map((c) => ({
      id: c.id,
      content_url: c.content_url,
      title: c.title,
      alexandria_url: c.alexandria_url,
      file_type: c.file_type,
      mime_type: c.mime_type,
      width: c.width,
      height: c.height,
      start_ms: excerptRange(c, workflow.ranges[c.id]?.[0] ?? 0, visibility[c.id])[0],
      end_ms: excerptRange(c, workflow.ranges[c.id]?.[0] ?? 0, visibility[c.id])[1],
      performers: c.performers,
      tags: c.tags
    }));
    const body = { ...opts, clips: selections } as RenderRequest;
    loading = true;
    error = null;
    try {
      const { id } = await api.render(body);
      workflow.jobId = id;
      saveWorkflow();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
</script>

{#if workflow.clips !== null && workflow.selected.length >= 2}
  {#if error}<div class="mb-6 alert alert-error" role="alert">{error}</div>{/if}
  {#if loading}<div class="mb-6 flex items-center gap-3 text-sm">
      <span class="loading loading-sm loading-spinner"></span> Working on it…
    </div>{/if}
  <a class="btn mb-5 -ml-3 btn-ghost btn-sm" href={resolve('/clips')}>← Back to clips</a>
  <RenderForm {onRender} />
  {#if workflow.jobId}<RenderProgress id={workflow.jobId} />{/if}
{/if}
