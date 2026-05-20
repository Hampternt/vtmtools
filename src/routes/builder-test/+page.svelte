<script lang="ts">
  import ValueSelector from '$lib/components/builder/ValueSelector.svelte';
  import AppliesToPicker from '$lib/components/builder/AppliesToPicker.svelte';
  import ItemBuilderShell from '$lib/components/builder/ItemBuilderShell.svelte';
  import CardComposer from '$lib/components/builder/CardComposer.svelte';
  let v = $state(3);
  let p = $state<string[]>([]);
  let shellOpen = $state(false);
  let composerOpen = $state(false);
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

  <section style="margin-top: 2rem;">
    <h2>ItemBuilderShell</h2>
    <button onclick={() => shellOpen = true}>Open shell</button>
    <ItemBuilderShell
      open={shellOpen}
      title="Smoke test"
      onSave={() => { alert('Save'); shellOpen = false; }}
      onCancel={() => { shellOpen = false; }}
    >
      <div style="padding: 1rem;">Hello from inside the shell. Ctrl+Enter saves. Esc cancels.</div>
    </ItemBuilderShell>
  </section>

  <section style="margin-top: 2rem;">
    <h2>CardComposer (inside Shell)</h2>
    <button onclick={() => composerOpen = true}>Open composer demo</button>
    <ItemBuilderShell
      open={composerOpen}
      title="Composer smoke test"
      onSave={() => { composerOpen = false; }}
      onCancel={() => { composerOpen = false; }}
    >
      <CardComposer
        queueValues={['next', 'then', 'finally']}
      >
        {#snippet cardRows()}
          <div style="padding: 0.5rem; color: var(--text-secondary);">Card rows placeholder — slots in Task 12.</div>
        {/snippet}
        {#snippet spotlight()}
          <div style="text-align: center; color: var(--text-secondary);">Spotlight placeholder — active editor in Task 13.</div>
        {/snippet}
      </CardComposer>
    </ItemBuilderShell>
  </section>
</div>
