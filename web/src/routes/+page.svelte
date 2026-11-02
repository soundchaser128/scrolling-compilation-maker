<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { api } from '#lib/api.js';
  import FilterForm from '#lib/components/FilterForm.svelte';
  import { workflow, saveWorkflow } from '#lib/workflow.svelte.js';

  let loading = $state(false);
  let error = $state<string | null>(null);

  async function onGenerate(filters: Record<string, unknown>) {
    loading = true;
    error = null;
    try {
      workflow.clips = await api.pool(filters);
      workflow.selected = [];
      workflow.ranges = {};
      workflow.jobId = null;
      saveWorkflow();
      await goto(resolve('/clips'));
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
</script>

{#if error}
  <div class="mb-6 alert alert-error" role="alert">{error}</div>
{/if}

<div class="mb-6 h-5 text-sm" role="status">
  {#if loading}
    <span class="loading loading-sm loading-spinner align-middle"></span> Working on it…
  {/if}
</div>

<FilterForm {onGenerate} />
