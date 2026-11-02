<script lang="ts">
  let {
    label,
    placeholder,
    value = $bindable(''),
    items = $bindable<string[]>([]),
    search
  }: {
    label: string;
    placeholder: string;
    value: string;
    items: string[];
    search: (query: string) => Promise<string[]>;
  } = $props();

  let id = $derived(`autocomplete-${label.toLowerCase()}`);
  let suggestions = $state<string[]>([]);
  let active = $state(-1);
  let open = $state(false);
  let focused = false;
  let timer: ReturnType<typeof setTimeout>;
  let request = 0;

  function clear() {
    clearTimeout(timer);
    request++;
    suggestions = [];
    active = -1;
    open = false;
  }

  function update(input: string) {
    value = input;
    clear();
    const query = input.split(',').at(-1)?.trim() ?? '';
    if (!query) return;
    const current = request;
    timer = setTimeout(async () => {
      try {
        const found = await search(query);
        if (current !== request) return;
        suggestions = found.filter((item) => !items.includes(item));
        open = focused && suggestions.length > 0;
      } catch {
        if (current === request) clear();
      }
    }, 200);
  }

  function add(input: string) {
    for (const item of input
      .split(',')
      .map((part) => part.trim())
      .filter(Boolean)) {
      if (!items.includes(item)) items = [...items, item];
    }
    value = '';
    clear();
  }

  function choose(item: string) {
    const parts = value.split(',');
    parts[parts.length - 1] = item;
    add(parts.join(','));
  }

  function keydown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown' && suggestions.length) {
      event.preventDefault();
      open = true;
      active = (active + 1) % suggestions.length;
    } else if (event.key === 'ArrowUp' && suggestions.length) {
      event.preventDefault();
      open = true;
      active = (active - 1 + suggestions.length) % suggestions.length;
    } else if (event.key === 'Enter') {
      event.preventDefault();
      if (open && active >= 0) choose(suggestions[active]);
      else add(value);
    } else if (event.key === 'Escape') {
      clear();
    }
  }

  $effect(() => () => {
    clearTimeout(timer);
    request++;
  });
</script>

<div class="field">
  <label for={id}>{label}</label>
  <div class="relative">
    <input
      {id}
      class="input w-full"
      {placeholder}
      role="combobox"
      aria-autocomplete="list"
      aria-expanded={open}
      aria-controls={`${id}-list`}
      aria-activedescendant={open && active >= 0 ? `${id}-option-${active}` : undefined}
      {value}
      oninput={(event) => update(event.currentTarget.value)}
      onkeydown={keydown}
      onblur={() => {
        focused = false;
        open = false;
      }}
      onfocus={() => {
        focused = true;
        if (suggestions.length) open = true;
      }}
    />
    {#if open}
      <ul
        id={`${id}-list`}
        role="listbox"
        class="menu absolute z-20 mt-1 max-h-60 w-full overflow-auto rounded-box border border-base-300 bg-base-100 p-1 shadow-lg"
      >
        {#each suggestions as suggestion, index}
          <li role="presentation">
            <button
              id={`${id}-option-${index}`}
              type="button"
              role="option"
              aria-selected={active === index}
              class:menu-active={active === index}
              onmousedown={(event) => event.preventDefault()}
              onclick={() => choose(suggestion)}>{suggestion}</button
            >
          </li>
        {/each}
      </ul>
    {/if}
  </div>
  <div class="flex flex-wrap gap-1">
    {#each items as item}<span class="badge badge-outline"
        >{item}<button
          type="button"
          aria-label={`Remove ${item}`}
          onclick={() => (items = items.filter((entry) => entry !== item))}>×</button
        ></span
      >{/each}
  </div>
</div>
