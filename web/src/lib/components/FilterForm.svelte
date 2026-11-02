<script lang="ts">
  import type { Orientation } from '$lib/types';
  import { api } from '$lib/api';
  let tagSuggestions = $state<string[]>([]);
  let personSuggestions = $state<string[]>([]);
  async function suggest(kind: 'tag' | 'person', value: string) {
    if (!value.trim()) return;
    try { const found = kind === 'tag' ? await api.tags(value) : await api.people(value); if (kind === 'tag') tagSuggestions = found; else personSuggestions = found; } catch { /* Manual entry remains available. */ }
  }

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
    tags: string[]; people: string[]; orientation: Orientation;
    with_images: boolean; max_clip_duration_ms: number; pool_size: number;
  };

  function addTag() {
    for (const v of tagInput.split(',').map(x => x.trim()).filter(Boolean)) if (!tags.includes(v)) tags = [...tags, v];
    tagInput = '';
  }
  function addPerson() {
    const v = personInput.trim();
    if (v && !people.includes(v)) people = [...people, v];
    personInput = '';
  }
</script>

<div class="filters grid grid-cols-1 gap-4 md:grid-cols-2">
  <label>Orientation
    <select class="select" bind:value={orientation}>
      <option value="portrait">Portrait</option>
      <option value="landscape">Landscape</option>
      <option value="square">Square</option>
      <option value="any">Any</option>
    </select>
  </label>

  <label>Tags (comma/enter separated)
    <input class="input w-full" list="tag-suggestions" bind:value={tagInput} oninput={() => suggest('tag', tagInput)} onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), addTag())} />
    <datalist id="tag-suggestions">{#each tagSuggestions as suggestion}<option value={suggestion}></option>{/each}</datalist>
    <span class="chips">{#each tags as t}<span class="chip">{t}<button class="btn btn-xs" aria-label={`Remove ${t}`} onclick={() => tags = tags.filter(x => x !== t)}>×</button></span>{/each}</span>
  </label>

  <label>People
    <input class="input w-full" list="person-suggestions" bind:value={personInput} oninput={() => suggest('person', personInput)} onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), addPerson())} />
    <datalist id="person-suggestions">{#each personSuggestions as suggestion}<option value={suggestion}></option>{/each}</datalist>
    <span class="chips">{#each people as p}<span class="chip">{p}<button class="btn btn-xs" aria-label={`Remove ${p}`} onclick={() => people = people.filter(x => x !== p)}>×</button></span>{/each}</span>
  </label>

  <label><input class="checkbox" type="checkbox" bind:checked={withImages} /> Include images</label>
  <label>Max clip duration (s) <input class="input" type="number" min="1" max="300" bind:value={maxDuration} /></label>
  <label>Pool size <input class="input" type="number" min="5" max="200" bind:value={poolSize} /></label>

  <button class="btn" onclick={() => onGenerate({
    tags: [...tags, ...tagInput.split(',').map(x => x.trim()).filter(Boolean)], people: [...people, ...personInput.split(',').map(x => x.trim()).filter(Boolean)], orientation, with_images: withImages,
    max_clip_duration_ms: maxDuration * 1000, pool_size: poolSize,
  })}>
    Generate pool
  </button>
</div>
