<script lang="ts">
  import type { Orientation } from '#lib/types.js';
  import { api } from '#lib/api.js';
  import Autocomplete from './Autocomplete.svelte';
  let tags = $state<string[]>([]);
  let people = $state<string[]>([]);
  let tagInput = $state('');
  let personInput = $state('');
  let orientation = $state<Orientation>('portrait');
  let withImages = $state(false);
  let maxDuration = $state(30);
  let poolSize = $state(50);

  let { onGenerate }: { onGenerate: (f: Filters) => void } = $props();

  export type Filters = {
    tags: string[];
    people: string[];
    orientation: Orientation;
    with_images: boolean;
    max_clip_duration_ms: number;
    pool_size: number;
  };
</script>

<div>
  <section class="card bg-base-100 shadow-sm card-border">
    <div class="card-body p-6 sm:p-8">
      <div class="grid gap-x-6 gap-y-5 md:grid-cols-2">
        <Autocomplete
          label="Tags"
          placeholder="e.g. nature, travel"
          bind:value={tagInput}
          bind:items={tags}
          search={api.tags}
        />
        <Autocomplete
          label="People"
          placeholder="Search people"
          bind:value={personInput}
          bind:items={people}
          search={api.people}
        />
        <label class="field"
          ><span>Orientation</span><select class="select w-full" bind:value={orientation}
            ><option value="portrait">Portrait</option><option value="landscape">Landscape</option
            ><option value="square">Square</option><option value="any">Any orientation</option
            ></select
          ></label
        >
        <label class="field"
          ><span>Maximum clip duration</span>
          <div class="input w-full">
            <input type="number" min="1" max="300" bind:value={maxDuration} /><span
              class="text-xs text-base-content/50">seconds</span
            >
          </div></label
        >
        <label class="field"
          ><span>Pool size</span>
          <div class="input w-full">
            <input type="number" min="5" max="200" bind:value={poolSize} /><span
              class="text-xs text-base-content/50">clips</span
            >
          </div></label
        >
        <label class="flex items-center gap-3 self-center rounded-box border border-base-300 p-4"
          ><input class="checkbox" type="checkbox" bind:checked={withImages} /><span
            ><span class="block text-sm font-bold">Include images</span></span
          ></label
        >
      </div>
      <div class="mt-7 border-t border-base-300 pt-6">
        <button
          class="btn w-full btn-neutral sm:w-auto"
          onclick={() =>
            onGenerate({
              tags: [
                ...tags,
                ...tagInput
                  .split(',')
                  .map((x) => x.trim())
                  .filter(Boolean)
              ],
              people: [
                ...people,
                ...personInput
                  .split(',')
                  .map((x) => x.trim())
                  .filter(Boolean)
              ],
              orientation,
              with_images: withImages,
              max_clip_duration_ms: maxDuration * 1000,
              pool_size: poolSize
            })}>Generate clip pool <span aria-hidden="true">→</span></button
        >
      </div>
    </div>
  </section>
</div>
