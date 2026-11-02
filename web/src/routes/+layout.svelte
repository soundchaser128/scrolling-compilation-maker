<script lang="ts">
  import '../app.css';
  import { page } from '$app/state';
  let { children } = $props();
  let step = $derived(page.route.id === '/clips' ? 2 : page.route.id === '/render' ? 3 : 1);
</script>

<svelte:head><title>Studio — Scrolling Compilation Maker</title></svelte:head>
<div class="min-h-screen bg-base-200 text-base-content">
  <main class="mx-auto max-w-7xl px-6 py-10 lg:px-10 lg:py-14">
    <div class="mb-10 flex flex-col justify-between gap-7 md:flex-row md:items-end">
      <div>
        <h1 class="mt-3 text-4xl font-black tracking-tight sm:text-5xl">
          {step === 1 ? 'Find clips' : step === 2 ? 'Pick clips' : 'Build compilation'}
        </h1>
      </div>
      <div class="stage-count">
        <span>0{step}</span><span class="text-base-content/30"> / 03</span>
      </div>
    </div>
    <ul
      class="steps mb-10 w-full max-w-2xl text-xs font-semibold sm:text-sm"
      aria-label="Workflow progress"
    >
      <li class="step step-neutral">Discover</li>
      <li class={['step', { 'step-neutral': step >= 2 }]}>Select clips</li>
      <li class={['step', { 'step-neutral': step >= 3 }]}>Render</li>
    </ul>
    {@render children()}
  </main>
</div>
