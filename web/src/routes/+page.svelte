<script lang="ts">
  import { api } from '$lib/api';
  import type { ClipCandidate, ClipSelection, RenderRequest } from '$lib/types';
  import FilterForm from '$lib/components/FilterForm.svelte';
  import ClipGrid from '$lib/components/ClipGrid.svelte';
  import RenderForm from '$lib/components/RenderForm.svelte';
  import RenderProgress from '$lib/components/RenderProgress.svelte';

  let step = $state<'filters' | 'pool' | 'render'>('filters');
  let clips = $state<ClipCandidate[]>([]);
  let selected = $state<Set<string>>(new Set());
  let ranges = $state<Record<string, [number, number]>>({});
  function onRange(id: string, start: number, end: number) { ranges[id] = [start, end]; }
  let loading = $state(false);
  let error = $state<string | null>(null);
  let jobId = $state<string | null>(null);

  function toggle(id: string) {
    selected = new Set(selected);
    selected.has(id) ? selected.delete(id) : selected.add(id);
    selected = selected;
  }

  async function onGenerate(filters: Record<string, unknown>) {
    loading = true; error = null;
    try {
      clips = await api.pool(filters);
      selected = new Set(); ranges = {};
      step = 'pool';
    } catch (e) { error = String(e); }
    finally { loading = false; }
  }

  async function onRender(opts: Record<string, unknown>) {
    const chosen = clips.filter((c) => selected.has(c.id));
    const selections: ClipSelection[] = chosen.map((c) => ({
      id: c.id, content_url: c.content_url, file_type: c.file_type,
      mime_type: c.mime_type,
      width: c.width, height: c.height,
      start_ms: ranges[c.id]?.[0] ?? 0, end_ms: ranges[c.id]?.[1] ?? c.duration_ms,
      performers: c.performers, tags: c.tags,
    }));
    const body = { ...opts, clips: selections } as RenderRequest;
    loading = true; error = null;
    try {
      const { id } = await api.render(body);
      jobId = id; step = 'render';
    } catch (e) { error = String(e); }
    finally { loading = false; }
  }
</script>

<main>
  <h1 class="text-3xl font-bold">Scrolling Compilation Maker</h1>
  <p>Choose clips, set their ranges, and make a scrolling video.</p>
  {#if error}<div class="alert alert-error" role="alert">{error}</div>{/if}
  {#if loading}<p>Working…</p>{/if}

  {#if step === 'filters'}
    <FilterForm {onGenerate} />
  {:else if step === 'pool'}
    <button class="btn" onclick={() => { step = 'filters'; }}>← Back to filters</button>
    {#if loading}<p>Loading…</p>{/if}
    <p>Select at least two clips to make a scrolling strip.</p>
    <ClipGrid {clips} {selected} {toggle} {ranges} {onRange} />
    <button class="btn" disabled={selected.size < 2} onclick={() => { step = 'render'; }}>
      Continue with {selected.size} clip{selected.size === 1 ? '' : 's'}
    </button>
  {:else}
    <button class="btn" onclick={() => { step = 'pool'; jobId = null; }}>Back to clips</button>
    <RenderForm {onRender} />
    {#if jobId}<RenderProgress id={jobId} />{/if}
  {/if}
</main>

<style>
  main { max-width: 1400px; margin: 0 auto; padding: 24px; }
  h1 { margin-bottom: .5rem; }
  main > p { margin-bottom: 2rem; }
  button { padding: 8px 14px; margin: 8px 0; }
  button:disabled { opacity: .4; }
</style>
