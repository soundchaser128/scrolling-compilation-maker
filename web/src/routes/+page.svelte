<script lang="ts">
  import { api } from '#lib/api.js';
  import type { ClipCandidate, ClipSelection, RenderRequest } from '#lib/types.js';
  import FilterForm from '#lib/components/FilterForm.svelte';
  import ClipGrid from '#lib/components/ClipGrid.svelte';
  import RenderForm from '#lib/components/RenderForm.svelte';
  import RenderProgress from '#lib/components/RenderProgress.svelte';

  let step = $state<'filters' | 'pool' | 'render'>('filters');
  let clips = $state<ClipCandidate[]>([]);
  let selected = $state<Set<string>>(new Set());
  let ranges = $state<Record<string, [number, number]>>({});
  function onRange(id: string, start: number, end: number) {
    ranges[id] = [start, end];
  }
  let loading = $state(false);
  let error = $state<string | null>(null);
  let jobId = $state<string | null>(null);

  function toggle(id: string) {
    selected = new Set(selected);
    selected.has(id) ? selected.delete(id) : selected.add(id);
    selected = selected;
  }

  async function onGenerate(filters: Record<string, unknown>) {
    loading = true;
    error = null;
    try {
      clips = await api.pool(filters);
      selected = new Set();
      ranges = {};
      step = 'pool';
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function onRender(opts: Record<string, unknown>) {
    const chosen = clips.filter((c) => selected.has(c.id));
    const selections: ClipSelection[] = chosen.map((c) => ({
      id: c.id,
      content_url: c.content_url,
      file_type: c.file_type,
      mime_type: c.mime_type,
      width: c.width,
      height: c.height,
      start_ms: ranges[c.id]?.[0] ?? 0,
      end_ms: ranges[c.id]?.[1] ?? c.duration_ms,
      performers: c.performers,
      tags: c.tags
    }));
    const body = { ...opts, clips: selections } as RenderRequest;
    loading = true;
    error = null;
    try {
      const { id } = await api.render(body);
      jobId = id;
      step = 'render';
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
</script>

<svelte:head><title>Studio — Scrolling Compilation Maker</title></svelte:head>
<div class="min-h-screen bg-base-200 text-base-content">
  <main class="mx-auto max-w-7xl px-6 py-10 lg:px-10 lg:py-14">
    <div class="mb-10 flex flex-col justify-between gap-7 md:flex-row md:items-end">
      <div>
        <h1 class="mt-3 text-4xl font-black tracking-tight sm:text-5xl">
          {step === 'filters'
            ? 'Find your clips.'
            : step === 'pool'
              ? 'Build your lineup.'
              : 'Make it move.'}
        </h1>
      </div>
      <div class="stage-count">
        <span>0{step === 'filters' ? '1' : step === 'pool' ? '2' : '3'}</span><span
          class="text-base-content/30"
        >
          / 03</span
        >
      </div>
    </div>
    <ul
      class="steps mb-10 w-full max-w-2xl text-xs font-semibold sm:text-sm"
      aria-label="Workflow progress"
    >
      <li class="step step-neutral">Discover</li>
      <li class:step-neutral={step !== 'filters'} class="step">Select clips</li>
      <li class:step-neutral={step === 'render'} class="step">Render</li>
    </ul>
    {#if error}<div class="mb-6 alert alert-error" role="alert">{error}</div>{/if}
    {#if loading}<div class="mb-6 flex items-center gap-3 text-sm">
        <span class="loading loading-sm loading-spinner"></span> Working on it…
      </div>{/if}
    {#if step === 'filters'}
      <FilterForm {onGenerate} />
    {:else if step === 'pool'}
      <div class="mb-6 flex flex-wrap items-end justify-between gap-4">
        <div>
          <button
            class="btn mb-3 -ml-3 btn-ghost btn-sm"
            onclick={() => {
              step = 'filters';
            }}>← Back to filters</button
          >
          <h2 class="text-2xl font-bold">
            Your clip pool <span class="text-base-content/35">/ {clips.length}</span>
          </h2>
        </div>
        <button
          class="btn btn-neutral"
          disabled={selected.size < 2 || loading}
          onclick={() => {
            step = 'render';
          }}
          >Continue with {selected.size} clip{selected.size === 1 ? '' : 's'}
          <span aria-hidden="true">→</span></button
        >
      </div>
      {#if clips.length}<ClipGrid {clips} {selected} {toggle} {ranges} {onRange} />{:else}<div
          class="card bg-base-100 p-10 text-center text-base-content/60 card-border"
        >
          No clips found. Try changing your filters.
        </div>{/if}
    {:else}
      <button
        class="btn mb-5 -ml-3 btn-ghost btn-sm"
        onclick={() => {
          step = 'pool';
          jobId = null;
        }}>← Back to clips</button
      >
      <RenderForm {onRender} />
      {#if jobId}<RenderProgress id={jobId} />{/if}
    {/if}
  </main>
</div>
