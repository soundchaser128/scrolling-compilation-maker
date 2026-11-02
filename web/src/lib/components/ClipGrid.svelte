<script lang="ts">
  import type { ClipCandidate } from '#lib/types.js';
  import { estimateVisibility, type TimingSettings } from '#lib/timing.js';
  import ClipCard from './ClipCard.svelte';
  let {
    clips,
    selected,
    toggle,
    ranges,
    onRange,
    timing
  }: {
    timing: TimingSettings;
    clips: ClipCandidate[];
    selected: Set<string>;
    toggle: (id: string) => void;
    ranges: Record<string, [number, number]>;
    onRange: (id: string, start: number, end: number) => void;
  } = $props();
  let estimates = $derived.by(() => {
    const chosen = clips.filter((c) => selected.has(c.id));
    const result = estimateVisibility(chosen, timing);
    for (const clip of clips) {
      if (!selected.has(clip.id)) {
        result[clip.id] = estimateVisibility(
          clips.filter((c) => selected.has(c.id) || c.id === clip.id),
          timing
        )[clip.id];
      }
    }
    return result;
  });
</script>

<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
  {#each clips as clip (clip.id)}
    <ClipCard
      {clip}
      visibility={estimates[clip.id]}
      selected={selected.has(clip.id)}
      range={ranges[clip.id] ?? [0, clip.duration_ms]}
      onToggle={() => toggle(clip.id)}
      onRange={(start, end) => onRange(clip.id, start, end)}
    />
  {/each}
</div>
