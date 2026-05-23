<script lang="ts">
  // No imports — just static links
</script>

<svelte:head><title>Builder routes — smoke test</title></svelte:head>

<main class="builder-test">
  <h1>Builder routes</h1>
  <p class="hint">Smoke-test landing page. Click any link to verify the route mounts cleanly.</p>
  <ul>
    <li><a href="/builder/advantage/new">New advantage</a></li>
    <li><a href="/builder/template/new">New status template</a></li>
  </ul>
  <p class="hint">Per-character modifier (`/builder/modifier/[id]`) requires an existing modifier id; create one via the character flow first.</p>
</main>

<style>
.builder-test {
  width: min(36rem, 100% - 2rem);
  margin: 2rem auto;
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}
.builder-test h1 {
  font-family: 'Spartan', sans-serif;
  font-weight: 200;
  font-size: 1.2rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--accent-amber);
}
.builder-test .hint {
  font-size: 0.7rem;
  color: var(--text-muted);
  font-family: 'Oswald', sans-serif;
}
.builder-test ul { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 0.4rem; }
.builder-test li a {
  display: block;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--outline-color);
  background: rgba(0, 0, 0, 0.2);
  color: var(--accent-amber);
  text-decoration: none;
  font-family: 'Oswald', sans-serif;
  font-size: 0.85rem;
  letter-spacing: 0.05em;
}
.builder-test li a:hover {
  background: rgba(217, 165, 96, 0.08);
  border-color: var(--accent-amber);
}
</style>
