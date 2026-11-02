<script lang="ts">
 import type { ClipCandidate } from '$lib/types';
 import ClipCard from './ClipCard.svelte';
 let { clips, selected, toggle, ranges, onRange }: { clips: ClipCandidate[]; selected: Set<string>; toggle: (id: string) => void; ranges: Record<string, [number, number]>; onRange: (id: string, start: number, end: number) => void } = $props();
</script>
<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4 xl:grid-cols-5">
 {#each clips as clip (clip.id)}
  <ClipCard {clip} selected={selected.has(clip.id)} range={ranges[clip.id] ?? [0, clip.duration_ms]} onToggle={() => toggle(clip.id)} onRange={(start, end) => onRange(clip.id, start, end)} />
 {/each}
</div>
