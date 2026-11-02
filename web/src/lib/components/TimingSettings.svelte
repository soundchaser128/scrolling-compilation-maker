<script lang="ts">
  import { workflow, saveWorkflow } from '#lib/workflow.svelte.js';
  import { clipWidth } from '#lib/timing.js';

  let chosen = $derived((workflow.clips ?? []).filter((c) => workflow.selected.includes(c.id)));
  let distance = $derived(
    chosen.reduce((sum, clip) => sum + clipWidth(clip, workflow.timing), 0) -
      workflow.timing.viewport_width
  );
  let aspect = $derived(`${workflow.timing.viewport_width}:${workflow.timing.viewport_height}`);

  function changed() {
    workflow.jobId = null;
    saveWorkflow();
  }

  function setAspect(event: Event) {
    const [width, height] = (event.currentTarget as HTMLSelectElement).value.split(':').map(Number);
    workflow.timing.viewport_width = width;
    workflow.timing.viewport_height = height;
    changed();
  }
</script>

<div class="mb-6 rounded-box border border-base-300 bg-base-100 p-5">
  <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
    <label class="field"
      ><span>Compilation duration (seconds)</span>
      <input
        class="input w-full"
        type="number"
        min="1"
        max="120"
        step="1"
        bind:value={workflow.timing.duration_secs}
        onchange={changed}
      />
    </label>
    <label class="field"
      ><span>Output aspect ratio</span>
      <select class="select w-full" value={aspect} onchange={setAspect}>
        <option value="1080:1920">9:16 · Portrait</option>
        <option value="1920:1080">16:9 · Landscape</option>
        <option value="1080:1080">1:1 · Square</option>
      </select>
    </label>
    <label class="field"
      ><span>Scroll movement</span>
      <select class="select w-full" bind:value={workflow.timing.easing} onchange={changed}>
        <option value="linear">Linear</option><option value="ease">Ease</option>
      </select>
    </label>
    <label class="field"
      ><span>Crop each clip</span>
      <select class="select w-full" bind:value={workflow.timing.crop} onchange={changed}>
        <option value="">Original aspect ratio</option><option value="9:16">9:16</option>
        <option value="16:9">16:9</option><option value="1:1">1:1</option>
      </select>
    </label>
  </div>
  <p class="mt-3 text-sm text-base-content/70">
    {chosen.length} selected · {distance > 0 && workflow.timing.duration_secs > 0
      ? `${(distance / workflow.timing.duration_secs).toFixed(0)} px/s average scroll speed`
      : 'Select enough clips to fill more than the output width.'}
    Screen time includes partial entrance and exit. Unselected clips show an estimate if added.
  </p>
</div>
