<script lang="ts">
  import type { Codec, Effort, Easing, Quality } from '$lib/types';

  let codec = $state<Codec>('x264');
  let quality = $state<Quality>('medium');
  let effort = $state<Effort>('medium');
  let easing = $state<Easing>('linear');
  let gpu = $state(false);
  let text = $state<'performers' | 'tags' | 'none'>('none');
  let crop = $state('');
  let song = $state('');
  let duration = $state(60);

  let { onRender }: { onRender: (o: Record<string, unknown>) => void } = $props();

  function submit() {
    onRender({
      codec, quality, effort, easing, gpu,
      text: text === 'none' ? null : text,
      crop: crop || null,
      song: song || null,
      duration_secs: duration,
      viewport_width: 1080,
      viewport_height: 1920,
    });
  }
</script>

<section class="card card-border bg-base-100 shadow-sm"><div class="card-body p-6 sm:p-8"><div class="eyebrow">03 / RENDER</div><h2 class="text-2xl font-bold">Output settings</h2><div class="grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
<label class="field"><span>Duration</span><div class="input w-full"><input type="number" bind:value={duration} min="5" max="120" /><span class="text-xs text-base-content/50">seconds</span></div></label>
<label class="field"><span>Codec</span><select class="select w-full" bind:value={codec}><option value="x264">H.264</option><option value="hevc">HEVC</option><option value="av1">AV1</option></select></label>
<label class="field"><span>Quality</span><select class="select w-full" bind:value={quality}><option value="low">Low</option><option value="medium">Medium</option><option value="high">High</option><option value="veryhigh">Very High</option></select></label>
<label class="field"><span>Encoding effort</span><select class="select w-full" bind:value={effort}><option value="low">Low</option><option value="medium">Medium</option><option value="high">High</option></select></label>
<label class="field"><span>Movement</span><select class="select w-full" bind:value={easing}><option value="linear">Linear</option><option value="ease">Ease</option></select></label>
<label class="field"><span>Overlay text</span><select class="select w-full" bind:value={text}><option value="none">None</option><option value="performers">Performers</option><option value="tags">Tags</option></select></label>
<label class="field"><span>Crop aspect</span><input class="input w-full" bind:value={crop} placeholder="Optional, e.g. 9:16" /></label>
<label class="field"><span>Song URL</span><input class="input w-full" bind:value={song} placeholder="Optional" /></label>
<label class="flex items-center gap-3 self-end rounded-box border border-base-300 p-3"><input class="checkbox" type="checkbox" bind:checked={gpu} /><span class="text-sm font-bold">GPU encoding</span></label>
</div><div class="mt-7 border-t border-base-300 pt-6"><button class="btn btn-neutral" onclick={submit}>Render compilation <span aria-hidden="true">→</span></button></div></div></section>
