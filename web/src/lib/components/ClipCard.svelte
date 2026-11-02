<script lang="ts">
  import type { ClipCandidate } from '#lib/types.js';
  import { api } from '#lib/api.js';
  import { excerptRange, type Visibility } from '#lib/timing.js';

  let {
    clip,
    selected,
    range,
    visibility,
    onToggle,
    onRange
  }: {
    clip: ClipCandidate;
    selected: boolean;
    range: [number, number];
    visibility?: Visibility;
    onToggle: () => void;
    onRange: (start: number, end: number) => void;
  } = $props();

  let video: HTMLVideoElement | undefined = $state();
  let currentTime = $state(0);
  let ready = $state(false);
  let looping = $state(false);
  let playbackError = $state('');
  let internalSeek = false;
  const isVideo = $derived(clip.file_type === 'video');
  const segment = $derived(excerptRange(clip, range[0], visibility));
  const startMs = $derived(segment[0]);
  const endMs = $derived(segment[1]);
  const fmt = (ms: number) => `${(ms / 1000).toFixed(2)}s`;

  function seekStart() {
    if (!video) return;
    internalSeek = true;
    video.currentTime = startMs / 1000;
  }

  async function preview() {
    if (!video || !ready) return;
    looping = true;
    playbackError = '';
    seekStart();
    try {
      await video.play();
    } catch {
      playbackError = 'Playback could not start. Use the video controls to try again.';
    }
  }

  function setStart() {
    if (!video || !ready) return;
    const start = Math.min(Math.round(video.currentTime * 1000), Math.max(0, clip.duration_ms - 1));
    onRange(start, excerptRange(clip, start, visibility)[1]);
    void preview();
  }

  function checkBoundary() {
    if (!video || !looping || video.seeking) return;
    if (video.currentTime * 1000 >= endMs || video.currentTime * 1000 < startMs) {
      seekStart();
      if (video.ended)
        void video.play().catch(() => {
          playbackError = 'Use the video controls to resume playback.';
        });
    }
  }

  function onSeeking() {
    if (!internalSeek) looping = false;
    internalSeek = false;
  }

  // Restart when selection/settings change the excerpt, including while paused.
  $effect(() => {
    const start = startMs;
    const end = endMs;
    if (looping && video && ready && end > start) {
      internalSeek = true;
      video.currentTime = start / 1000;
    }
  });

  $effect(() => {
    if (!looping) return;
    let frame: number;
    const tick = () => {
      checkBoundary();
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  });
</script>

<div class="card" class:selected>
  <div class="media">
    {#if isVideo}
      <video
        bind:this={video}
        src={clip.content_url}
        poster={api.thumbUrl(clip.content_url, 0)}
        preload="metadata"
        controls
        muted
        bind:currentTime
        playsinline
        onloadedmetadata={() => {
          ready = true;
        }}
        onseeking={onSeeking}
        ontimeupdate={checkBoundary}
        onended={checkBoundary}
      ></video>
    {:else}
      <img src={clip.content_url} alt="" />
    {/if}
  </div>

  <button
    class={['btn btn-primary btn-sm', !selected && 'btn-outline']}
    aria-pressed={selected}
    onclick={onToggle}>{selected ? 'Selected' : 'Select'}</button
  >
  <div class="row">
    <span class="title"
      >{clip.performers.join(', ') || clip.tags.slice(0, 3).join(', ') || 'untitled'}</span
    >
    <span class="popularity">★ {clip.popularity.toFixed(1)}</span>
  </div>

  {#if isVideo}
    <div class="trim">
      <div class="flex flex-wrap gap-2">
        <button class="btn btn-sm" disabled={!ready} onclick={setStart}>Set start</button>
        <button class="btn btn-sm" disabled={!ready} onclick={preview}>Preview loop</button>
      </div>
      <div class="times">
        Position {fmt(currentTime * 1000)} · Start {fmt(startMs)} → {fmt(endMs)}
      </div>
      <label class="flex items-center gap-2">
        <input
          class="checkbox checkbox-sm"
          type="checkbox"
          bind:checked={looping}
          disabled={!ready}
        />
        Loop excerpt
      </label>
      {#if visibility && endMs - startMs < visibility.duration * 1000 - 1}
        <p class="text-xs text-warning">
          Only {fmt(endMs - startMs)} remains in the source; this excerpt repeats while visible.
        </p>
      {/if}
      {#if playbackError}<p class="text-xs text-error" role="alert">{playbackError}</p>{/if}
    </div>
  {/if}

  <div class="text-sm text-base-content/70">
    {#if visibility}
      ≈ {visibility.duration.toFixed(1)}s visible{selected ? '' : ' if added'}
      <div class="text-xs">
        In compilation: {visibility.enter.toFixed(1)}s → {visibility.exit.toFixed(1)}s
      </div>
    {/if}
  </div>
</div>

<style>
  .card {
    border: 1px solid var(--color-base-300);
    border-radius: 1rem;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    background: var(--color-base-100);
  }
  .card.selected {
    border-color: var(--color-neutral);
    box-shadow: 0 0 0 2px var(--color-neutral);
  }
  .media {
    position: relative;
    aspect-ratio: 9/16;
    background: #171d19;
    border-radius: 0.6rem;
    overflow: hidden;
  }
  video,
  img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .row {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 13px;
    font-weight: 700;
  }
  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .popularity {
    color: #8b7436;
    white-space: nowrap;
  }
  .trim {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
  }
  .times,
  .tags {
    color: color-mix(in oklab, var(--color-base-content) 55%, transparent);
    font-size: 11px;
  }
  .tags {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
