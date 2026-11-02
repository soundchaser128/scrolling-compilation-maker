<script lang="ts">
  import type { ClipCandidate } from '$lib/types';
  import { api } from '$lib/api';
  import { untrack } from 'svelte';

  let { clip, selected, range, onToggle, onRange }: { clip: ClipCandidate; selected: boolean; range: [number, number]; onToggle: () => void; onRange: (start: number, end: number) => void } = $props();

  let startMs = $state(untrack(() => range[0]));
  let endMs = $state(untrack(() => range[1]));
  let playing = $state(false);
  let video: HTMLVideoElement | undefined = $state();

  function updateRange() {
    if (startMs >= endMs) startMs = Math.max(0, endMs - 100);
    onRange(startMs, endMs);
  }

  function playSegment() {
    if (!video) return;
    video.currentTime = startMs / 1000;
    void video.play();
    playing = true;
  }

  function onTimeUpdate() {
    if (video && video.currentTime * 1000 >= endMs) {
      video.pause();
      playing = false;
    }
  }

  function onEnded() { playing = false; }

  const isVideo = $derived(clip.file_type === 'video');
  const fmt = (ms: number) => `${(ms / 1000).toFixed(1)}s`;
</script>

<div class="card" class:selected>
  <div class="media">
    {#if isVideo}
      <video
        bind:this={video}
        src={clip.content_url}
        poster={api.thumbUrl(clip.content_url, 0)}
        preload="metadata"
        muted
        playsinline
        ontimeupdate={onTimeUpdate}
        onended={onEnded}
      ></video>
    {:else}
      <img src={clip.content_url} alt="" />
    {/if}
    {#if isVideo && !playing}
      <button class="play" onclick={playSegment}>▶</button>
    {/if}
  </div>

  <button class="btn btn-sm" aria-pressed={selected} onclick={onToggle}>{selected ? "Selected" : "Select clip"}</button>
  <div class="row">
    <span class="title">{clip.performers.join(', ') || clip.tags.slice(0, 3).join(', ') || 'untitled'}</span>
    <span class="popularity">★ {clip.popularity.toFixed(1)}</span>
  </div>

  {#if isVideo}
    <div class="trim">
      <label>Start <input class="range range-sm" type="range" min="0" max={clip.duration_ms} step="100" bind:value={startMs} onchange={updateRange} /></label>
      <label>End <input class="range range-sm" type="range" min="0" max={clip.duration_ms} step="100" bind:value={endMs} onchange={updateRange} /></label>
      <div class="times">{fmt(startMs)} → {fmt(endMs)}</div>
    </div>
  {/if}

  <div class="tags">{clip.tags.join(' · ')}</div>
</div>

<style>
  .card { border: 1px solid #333; border-radius: 8px; padding: 8px; display: flex; flex-direction: column; gap: 6px; }
  .card.selected { border-color: #4caf50; box-shadow: 0 0 0 2px #4caf5066; }
  .media { position: relative; aspect-ratio: 9/16; background: #111; border-radius: 6px; overflow: hidden; }
  video, img { width: 100%; height: 100%; object-fit: contain; }
  .play { position: absolute; inset: 0; margin: auto; width: 48px; height: 48px; border-radius: 50%; border: none; background: #000a; color: #fff; font-size: 20px; cursor: pointer; }
  .row { display: flex; justify-content: space-between; font-size: 13px; }
  .title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .popularity { color: #f9a825; white-space: nowrap; }
  .trim { display: flex; flex-direction: column; gap: 4px; font-size: 12px; }
  .trim input { width: 100%; }
  .times { color: #888; }
  .tags { font-size: 11px; color: #aaa; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
