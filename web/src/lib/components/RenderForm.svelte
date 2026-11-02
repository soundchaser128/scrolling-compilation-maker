<script lang="ts">
  import type { Codec, Effort, Quality } from '#lib/types.js';

  import TimingSettings from './TimingSettings.svelte';
  import { workflow } from '#lib/workflow.svelte.js';

  let codec = $state<Codec>('x264');
  let quality = $state<Quality>('medium');
  let effort = $state<Effort>('medium');
  let gpu = $state(false);
  let text = $state<'performers' | 'tags' | 'none'>('none');
  let song = $state('');

  let { onRender }: { onRender: (o: Record<string, unknown>) => void } = $props();

  function submit() {
    onRender({
      codec,
      quality,
      effort,
      ...workflow.timing,
      gpu,
      text: text === 'none' ? null : text,
      crop: workflow.timing.crop || null,
      song: song || null
    });
  }
</script>

<section class="card bg-base-100 shadow-sm card-border">
  <div class="card-body p-6 sm:p-8">
    <div class="eyebrow">03 / RENDER</div>
    <h2 class="text-2xl font-bold">Output settings</h2>
    <TimingSettings />
    <div class="grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
      <label class="field"
        ><span>Codec</span><select class="select w-full" bind:value={codec}
          ><option value="x264">H.264</option><option value="hevc">HEVC</option><option value="av1"
            >AV1</option
          ></select
        ></label
      >
      <label class="field"
        ><span>Quality</span><select class="select w-full" bind:value={quality}
          ><option value="low">Low</option><option value="medium">Medium</option><option
            value="high">High</option
          ><option value="veryhigh">Very High</option></select
        ></label
      >
      <label class="field"
        ><span>Encoding effort</span><select class="select w-full" bind:value={effort}
          ><option value="low">Low</option><option value="medium">Medium</option><option
            value="high">High</option
          ></select
        ></label
      >
      <label class="field"
        ><span>Overlay text</span><select class="select w-full" bind:value={text}
          ><option value="none">None</option><option value="performers">Performers</option><option
            value="tags">Tags</option
          ></select
        ></label
      >
      <label class="field"
        ><span>Song URL</span><input
          class="input w-full"
          bind:value={song}
          placeholder="Optional"
        /></label
      >
      <label class="flex items-center gap-3 self-end rounded-box border border-base-300 p-3"
        ><input class="checkbox" type="checkbox" bind:checked={gpu} /><span
          class="text-sm font-bold">GPU encoding</span
        ></label
      >
    </div>
    <div class="mt-7 border-t border-base-300 pt-6">
      <button
        class="btn btn-neutral"
        onclick={submit}
        disabled={!(
          workflow.timing.duration_secs >= 1 &&
          workflow.timing.duration_secs <= 120 &&
          Number.isInteger(workflow.timing.duration_secs)
        )}>Render compilation <span aria-hidden="true">→</span></button
      >
    </div>
  </div>
</section>
