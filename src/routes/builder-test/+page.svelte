<script lang="ts">
  import ValueSelector from '$lib/components/builder/ValueSelector.svelte';
  import AppliesToPicker from '$lib/components/builder/AppliesToPicker.svelte';
  let v = $state(3);
  let p = $state<string[]>([]);
</script>

<div style="padding: 2rem; color: var(--text-primary); background: var(--bg-page); min-height: 100vh;">
  <h1>Builder Primitives Smoke Test</h1>

  <section style="margin-top: 2rem;">
    <h2>ValueSelector</h2>
    <ValueSelector value={v} onChange={(n) => v = n} />
    <p>Current: {v}</p>
  </section>

  <section style="margin-top: 2rem;">
    <h2>AppliesToPicker</h2>
    <AppliesToPicker paths={p} onChange={(next) => p = next} />
    <p>Paths: {JSON.stringify(p)}</p>
  </section>
</div>
