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

<div class="renderform grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
  <label>Duration <input class="input" type="number" bind:value={duration} min="5" max="120" />s</label>
  <label>Codec
    <select class="select" bind:value={codec}><option value="x264">H.264</option><option value="hevc">HEVC</option><option value="av1">AV1</option></select>
  </label>
  <label>Quality
    <select class="select" bind:value={quality}><option value="low">Low</option><option value="medium">Medium</option><option value="high">High</option><option value="veryhigh">Very High</option></select>
  </label>
  <label>Effort
    <select class="select" bind:value={effort}><option value="low">Low</option><option value="medium">Medium</option><option value="high">High</option></select>
  </label>
  <label>Easing
    <select class="select" bind:value={easing}><option value="linear">Linear</option><option value="ease">Ease</option></select>
  </label>
  <label><input class="checkbox" type="checkbox" bind:checked={gpu} /> GPU encoding</label>
  <label>Overlay text
    <select class="select" bind:value={text}><option value="none">None</option><option value="performers">Performers</option><option value="tags">Tags</option></select>
  </label>
  <label>Crop aspect (e.g. 9:16) <input class="input" bind:value={crop} placeholder="optional" /></label>
  <label>Song URL <input class="input" bind:value={song} placeholder="optional" /></label>
  <button class="btn" onclick={submit}>Render</button>
</div>
