<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { onMount } from 'svelte';
  import ClipGrid from '#lib/components/ClipGrid.svelte';
  import { workflow, saveWorkflow } from '#lib/workflow.svelte.js';

  let selected = $derived(new Set(workflow.selected));

  onMount(() => {
    if (workflow.clips === null) void goto(resolve('/'), { replaceState: true });
  });

  function toggle(id: string) {
    workflow.selected = selected.has(id)
      ? workflow.selected.filter((item) => item !== id)
      : [...workflow.selected, id];
    workflow.jobId = null;
    saveWorkflow();
  }

  function onRange(id: string, start: number, end: number) {
    workflow.ranges[id] = [start, end];
    workflow.jobId = null;
    saveWorkflow();
  }
</script>

{#if workflow.clips !== null}
  <div class="mb-6 flex flex-wrap items-end justify-between gap-4">
    <div>
      <a class="btn mb-3 -ml-3 btn-ghost btn-sm" href={resolve('/')}>← Back to filters</a>
      <h2 class="text-2xl font-bold">
        Your clip pool <span class="text-base-content/35">/ {workflow.clips.length}</span>
      </h2>
    </div>
    <button
      class="btn btn-neutral"
      disabled={selected.size < 2}
      onclick={() => goto(resolve('/render'))}
    >
      Continue with {selected.size} clip{selected.size === 1 ? '' : 's'}
      <span aria-hidden="true">→</span>
    </button>
  </div>
  {#if workflow.clips.length}
    <ClipGrid clips={workflow.clips} {selected} {toggle} ranges={workflow.ranges} {onRange} />
  {:else}
    <div class="card bg-base-100 p-10 text-center text-base-content/60 card-border">
      No clips found. Try changing your filters.
    </div>
  {/if}
{/if}
