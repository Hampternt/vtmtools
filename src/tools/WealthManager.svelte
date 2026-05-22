<script lang="ts">
  // Top-level Wealth tool. Lets the GM pick a saved character, then
  // either enables wealth tracking (creates a wallet row) or shows
  // three placeholder panels filled in by Tasks 8/9/10.
  //
  // Character-list source: the saved_characters table via the shared
  // `savedCharacters` runes store. There is no separate `listCharacters`
  // API in this repo; saved characters ARE the character list (see
  // src/store/savedCharacters.svelte.ts). Their numeric `id` is what
  // `wealth_*` Tauri commands expect for `character_id`.
  //
  // No invoke() in this component — all IPC goes through
  // src/lib/wealth/api.ts per ARCHITECTURE.md §4.

  import { onMount } from 'svelte';
  import { savedCharacters } from '../store/savedCharacters.svelte';
  import type { SavedCharacter } from '$lib/saved-characters/api';
  import type { Wallet, Asset, Bucket } from '../types';
  import {
    getWallet,
    enableWallet,
    disableWallet,
    adjustLiquid,
    adjustBanked,
    transferFunds,
    listAssets,
  } from '$lib/wealth/api';
  import WalletPanel from '$lib/components/wealth/WalletPanel.svelte';
  import AdjustModal from '$lib/components/wealth/AdjustModal.svelte';
  import TransferModal from '$lib/components/wealth/TransferModal.svelte';

  // -------- state
  let selectedId: number | null = $state(null);
  let wallet: Wallet | null = $state(null);
  let assets: Asset[] = $state([]);
  let loadingWallet = $state(false);
  let error: string | null = $state(null);

  // -------- modal state
  let adjustOpen = $state(false);
  let adjustBucket: Bucket = $state('liquid');
  let transferOpen = $state(false);

  // Derive character list from the store. Keeps in sync with bridge
  // updates / new saves automatically (the store re-fetches on
  // `bridge://characters-updated`).
  let characters = $derived<SavedCharacter[]>(savedCharacters.list);
  let loadingChars = $derived(savedCharacters.loading);

  // -------- lifecycle
  onMount(async () => {
    try {
      await savedCharacters.ensureLoaded();
      if (savedCharacters.list.length > 0 && selectedId == null) {
        selectedId = savedCharacters.list[0].id;
        await refreshWallet();
      }
    } catch (e) {
      error = String(e);
    }
  });

  // -------- helpers
  async function refreshWallet() {
    if (selectedId == null) {
      wallet = null;
      assets = [];
      return;
    }
    loadingWallet = true;
    error = null;
    try {
      wallet = await getWallet(selectedId);
      if (wallet != null) {
        await refreshAssets();
      } else {
        assets = [];
      }
    } catch (e) {
      error = String(e);
    } finally {
      loadingWallet = false;
    }
  }

  async function refreshAssets() {
    if (selectedId == null) {
      assets = [];
      return;
    }
    try {
      assets = await listAssets(selectedId);
    } catch (e) {
      error = String(e);
    }
  }

  async function onSelectCharacter(e: Event) {
    const v = (e.target as HTMLSelectElement).value;
    selectedId = v ? Number(v) : null;
    await refreshWallet();
  }

  async function onEnable() {
    if (selectedId == null) return;
    error = null;
    try {
      wallet = await enableWallet(selectedId);
      await refreshAssets();
    } catch (e) {
      error = String(e);
    }
  }

  async function onDisable() {
    if (selectedId == null) return;
    if (!confirm(
      'Disable wealth tracking for this character? This will permanently delete the wallet and all asset records. This cannot be undone.'
    )) return;
    error = null;
    try {
      await disableWallet(selectedId);
      wallet = null;
      assets = [];
    } catch (e) {
      error = String(e);
    }
  }

  // -------- adjust + transfer handlers
  function openAdjust(b: Bucket) {
    adjustBucket = b;
    adjustOpen = true;
  }

  function openTransfer() {
    transferOpen = true;
  }

  async function applyAdjust(delta: number) {
    if (selectedId == null) return;
    try {
      wallet = adjustBucket === 'liquid'
        ? await adjustLiquid(selectedId, delta)
        : await adjustBanked(selectedId, delta);
    } catch (e) {
      error = String(e);
    }
  }

  async function applyTransfer(from: Bucket, to: Bucket, amount: number) {
    if (selectedId == null) return;
    try {
      wallet = await transferFunds(selectedId, from, to, amount);
    } catch (e) {
      error = String(e);
    }
  }
</script>

<section class="wealth-manager">
  <header class="manager-head">
    <h2>Wealth</h2>
    <label class="char-picker">
      <span>Character:</span>
      <select onchange={onSelectCharacter} disabled={loadingChars || characters.length === 0}>
        {#each characters as c (c.id)}
          <option value={c.id} selected={c.id === selectedId}>{c.name}</option>
        {/each}
      </select>
    </label>
  </header>

  {#if error}
    <div class="error-banner" role="alert">{error}</div>
  {/if}

  {#if loadingChars && characters.length === 0}
    <p class="empty">Loading characters…</p>
  {:else if selectedId == null}
    <p class="empty">No characters available. Save a character first.</p>
  {:else if loadingWallet}
    <p class="empty">Loading wallet…</p>
  {:else if wallet == null}
    <div class="disabled-state">
      <p>This character is not using the wealth system.</p>
      <button type="button" class="enable-btn" onclick={onEnable}>
        Enable wealth tracking
      </button>
    </div>
  {:else}
    <div class="enabled-state">
      <WalletPanel
        wallet={wallet!}
        {assets}
        onOpenAdjust={openAdjust}
        onOpenTransfer={openTransfer}
      />
      <!-- Tasks 9/10 fill these panels. -->
      <div class="panel-placeholder">Assets panel — Task 9</div>
      <div class="panel-placeholder">Shop panel — Task 10</div>
      <button type="button" class="disable-btn" onclick={onDisable}>
        Disable wealth tracking
      </button>

      <AdjustModal
        bind:open={adjustOpen}
        bucket={adjustBucket}
        wallet={wallet!}
        onApply={applyAdjust}
      />
      <TransferModal
        bind:open={transferOpen}
        wallet={wallet!}
        onApply={applyTransfer}
      />
    </div>
  {/if}
</section>

<style>
  /* Token substitutions vs the plan's draft:
       --color-bg          -> --bg-base
       --color-surface     -> --bg-card
       --color-text        -> --text-primary
       --color-text-muted  -> --text-muted
       --color-border      -> --border-card
       --color-accent      -> --accent
       --color-danger      -> --accent  (red is THE accent in this repo)
       --color-danger-bg   -> --bg-active
       --space-*, --radius-*, --text-* — no such tokens; use rem literals
       to match existing tools (AdvantagesManager, GmScreen, etc).
     Per CLAUDE.md "rem not px" + colors via tokens only. */

  .wealth-manager {
    display: grid;
    gap: 1rem;
    padding: 1rem 1.25rem;
    align-items: start;
  }

  .manager-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
  }

  h2 {
    margin: 0;
    color: var(--accent);
    font-size: 1.4rem;
  }

  .char-picker {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    color: var(--text-muted);
    font-size: 0.82rem;
  }

  .char-picker select {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 0.3rem;
    padding: 0.4rem 0.5rem;
    color: var(--text-primary);
    font-size: 0.82rem;
    min-width: 12rem;
  }

  .char-picker select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .empty {
    color: var(--text-ghost);
    text-align: center;
    padding: 2rem;
    font-size: 0.82rem;
  }

  .disabled-state {
    color: var(--text-muted);
    text-align: center;
    padding: 1.5rem;
    border: 1px dashed var(--border-card);
    border-radius: 0.4rem;
    display: grid;
    align-items: start;
    gap: 0.75rem;
    background: var(--bg-card);
  }

  .disabled-state p {
    margin: 0;
  }

  .enable-btn {
    justify-self: center;
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--accent);
    border-radius: 0.3rem;
    padding: 0.5rem 1rem;
    font-size: 0.82rem;
    cursor: pointer;
  }

  .enable-btn:hover {
    color: var(--accent-bright);
  }

  .enabled-state {
    display: grid;
    gap: 1rem;
    align-items: start;
  }

  .panel-placeholder {
    padding: 1rem;
    border: 1px dashed var(--border-card);
    border-radius: 0.4rem;
    color: var(--text-muted);
    text-align: center;
    background: var(--bg-card);
    font-size: 0.82rem;
  }

  .disable-btn {
    justify-self: end;
    background: transparent;
    border: 1px solid var(--accent);
    color: var(--accent);
    border-radius: 0.3rem;
    padding: 0.4rem 0.75rem;
    font-size: 0.78rem;
    cursor: pointer;
  }

  .disable-btn:hover {
    background: var(--bg-active);
    color: var(--accent-bright);
  }

  .error-banner {
    padding: 0.6rem 0.75rem;
    border: 1px solid var(--accent);
    border-radius: 0.4rem;
    color: var(--accent);
    background: var(--bg-active);
    font-size: 0.82rem;
  }
</style>
