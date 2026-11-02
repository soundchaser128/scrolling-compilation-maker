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

{#if loading}
  <div class="mb-6 flex items-center gap-3 text-sm">
    <span class="loading loading-sm loading-spinner"></span> Working on it…
  </div>
{/if}

<FilterForm {onGenerate} />
