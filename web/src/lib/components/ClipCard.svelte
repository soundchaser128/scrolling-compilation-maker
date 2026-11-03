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
    <span class="title">{clip.title}</span>
    <span class="popularity">★ {clip.popularity.toFixed(1)}</span>
  </div>

  <div class="row">
    {clip.performers.join(', ')}
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

<style lang="postcss">
  @reference '../../app.css';

  .card {
    @apply flex flex-col gap-2.5 rounded-box border border-base-300 bg-base-100 p-2.5;
  }
  .card.selected {
    @apply border-neutral ring-2 ring-neutral;
  }
  .media {
    @apply relative -mx-2.5 -mt-2.5 aspect-9/16 overflow-hidden rounded-t-box;
    background: #171d19;
  }
  video,
  img {
    @apply h-full w-full object-contain;
  }
  .row {
    @apply flex justify-between gap-2 font-bold;
    font-size: 13px;
  }
  .title {
    @apply truncate;
  }
  .popularity {
    @apply whitespace-nowrap;
    color: #8b7436;
  }
  .trim {
    @apply flex flex-col gap-1.5;
    font-size: 12px;
  }
  .times {
    @apply text-base-content/55;
    font-size: 11px;
  }
</style>
