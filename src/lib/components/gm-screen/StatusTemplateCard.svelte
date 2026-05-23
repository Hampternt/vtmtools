<script lang="ts">
  import type { StatusTemplate } from '../../../types';

  /**
   * Discriminated input: lets the same card render both saved library
   * rows and unsaved builder previews. Saved rows carry a DB id plus
   * the wip flag; previews omit id (the builder hasn't issued one yet)
   * and pass `wip` alongside the payload.
   */
  type SavedInput   = { kind: 'saved';   data: StatusTemplate };
  type PreviewInput = {
    kind: 'preview';
    data: Omit<StatusTemplate, 'id'>;
    wip?: boolean;
  };

  interface Props {
    input: SavedInput | PreviewInput;
    /** Saved-mode "summary" line under the name (e.g. effect digest). */
    summary?: string;
    /** Saved-mode primary action — apply the template. */
    onapply?: () => void;
    /** Whether the apply button is disabled (e.g. no focused character). */
    applyDisabled?: boolean;
    /** Tooltip for the apply button. */
    applyTitle?: string;
    /** Saved-mode edit callback. Card has no IPC knowledge. */
    onedit?: () => void;
    /** Saved-mode delete callback. Card has no IPC knowledge.
     *  Wired by the caller (StatusPaletteDock) when deletion is supported.
     *  Idiom matches `onedit` (no id parameter — caller closes over the row). */
    onDelete?: () => void;
    /** Preview-mode click-to-edit. Ignored in saved mode. */
    onClick?: (target: HTMLElement) => void;
  }

  // `onedit` is intentionally NOT surfaced as a ✎ icon — spec §"Hover-reveal
  // edit affordance" scopes the inline edit icon to AdvantageCard only.
  // Click-the-card-to-edit IS supported in saved mode (the spec only forbids
  // the icon, not the click affordance), and matches AdvantageCard's read-
  // mode behavior of opening its editor on click. The saved-mode <article>
  // below wires its onclick / onkeydown to fire onedit when the click
  // target isn't an inner interactive element (delete icon, apply button).
  let {
    input,
    summary = '',
    onapply,
    applyDisabled = false,
    applyTitle = '',
    onedit,
    onDelete,
    onClick,
  }: Props = $props();

  const data = $derived(input.data);
  const name = $derived(data.name);
  const isPreview = $derived(input.kind === 'preview');
  const isWip = $derived(input.kind === 'saved' ? input.data.wip : (input.wip ?? false));
  const effectCount = $derived(data.effects?.length ?? 0);
  // StatusTemplate has no `sourceAttribution` field today; reserved as a
  // future hook (per Foundry helper library roadmap, imports may surface
  // attribution on this card too). Falsy today by design.
  const isImported = $derived(false);

  function onDeleteClick(e: Event) {
    e.stopPropagation();
    const ok = confirm(`Delete ${name}?`);
    if (ok) onDelete?.();
  }

  function onApplyClick(e: Event) {
    e.stopPropagation();
    onapply?.();
  }

  function onArticleClick(e: MouseEvent) {
    if (isPreview) {
      if (onClick) onClick(e.target as HTMLElement);
      return;
    }
    // Saved mode: card-body click → onedit, unless the click originated from
    // an inner interactive element (delete icon, apply button).
    if (!onedit) return;
    const t = e.target;
    if (t instanceof HTMLElement && t.closest('button')) return;
    onedit();
  }

  function onArticleKeydown(e: KeyboardEvent) {
    if (isPreview) return;
    if (!onedit) return;
    if (e.key !== 'Enter' && e.key !== ' ') return;
    // Only the article itself activates — inner buttons handle their own keys.
    if (e.target !== e.currentTarget) return;
    e.preventDefault();
    onedit();
  }

  const isSavedEditable = $derived(input.kind === 'saved' && !!onedit);
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<article
  class="vamp-card status-template"
  class:preview={isPreview}
  class:wip={isWip}
  class:editable={isSavedEditable}
  data-source={isImported ? 'fvtt' : 'native'}
  role={isSavedEditable ? 'button' : undefined}
  tabindex={isSavedEditable ? 0 : undefined}
  onclick={onArticleClick}
  onkeydown={isSavedEditable ? onArticleKeydown : undefined}
>
  {#if input.kind === 'saved' && onDelete}
    <button
      type="button"
      class="affordance delete-icon"
      aria-label={`Delete ${name}`}
      onclick={onDeleteClick}
    >🗑</button>
  {/if}

  <div class="frame">
    <header class="zone-title">
      <h3 data-stage="name">{name}</h3>
    </header>
    <section class="zone-content">
      {#if data.description || summary}
        <p class="description" data-stage="description">{data.description || summary}</p>
      {/if}
      {#if data.tags.length > 0}
        <div class="tag-row" data-stage="tags">
          {#each data.tags as t}
            <span class="tag">{t}</span>
          {/each}
        </div>
      {/if}
      <div class="hairline" aria-hidden="true"></div>
    </section>
    <footer class="zone-footer">
      <div class="detail" data-field="effects">
        <h6>EFFECTS</h6>
        <span class="value">{effectCount} effect{effectCount === 1 ? '' : 's'}</span>
      </div>
      <div class="detail" data-field="tags">
        <h6>TAGS</h6>
        <span class="value">{data.tags.length || '—'}</span>
      </div>
    </footer>
  </div>

  {#if input.kind === 'saved'}
    <!-- Apply action row (preserved primary action; below the three-zone
         frame). Apply is high-frequency in the GM Screen flow, so the
         spec's hover-reveal pattern would hurt discoverability — leave
         it visible. -->
    <div class="action-row">
      <button
        type="button"
        class="apply"
        onclick={onApplyClick}
        disabled={applyDisabled}
        title={applyTitle}
      >Apply</button>
    </div>
  {/if}
</article>

<style>
  .vamp-card {
    position: relative;
    width: 100%;
    max-width: 22rem;
    min-width: 14rem;
    font-family: 'Iowan Old Style', Garamond, serif;
    color: var(--text-primary);
    box-shadow: var(--shadow-medium);
  }

  .frame {
    border: 1px solid var(--outline-color);
    position: relative;
    background: var(--bg-card, #1d1c1d);
  }
  .frame::before {
    content: "";
    position: absolute;
    inset: 4px;
    border: 1px solid var(--outline-color);
    pointer-events: none;
    opacity: 0.35;
  }

  .zone-title {
    padding: 0.85rem 1rem 0.5rem;
    text-align: center;
  }
  .zone-title h3 {
    margin: 0;
    font-family: 'Spartan', sans-serif;
    font-weight: 200;
    font-size: 1.05rem;
    letter-spacing: 1.6px;
    line-height: 1.25;
    text-transform: uppercase;
  }

  .zone-content {
    padding: 0 1rem 0.55rem;
  }
  .zone-content .description {
    margin: 0 0 0.5rem;
    font-family: 'Oswald', sans-serif;
    font-weight: 200;
    font-size: 0.82rem;
    letter-spacing: 0.4px;
    line-height: 1.55;
    text-align: left;
  }
  .zone-content .tag-row {
    display: flex;
    justify-content: center;
    gap: 0.25rem;
    font-family: 'Spartan', sans-serif;
    font-size: 0.6rem;
    letter-spacing: 0.4px;
    color: var(--text-muted);
    text-transform: lowercase;
  }
  .zone-content .tag-row .tag::before { content: "· "; opacity: 0.5; }
  .zone-content .tag-row .tag:first-child::before { content: ""; }
  .zone-content .hairline {
    height: 1px;
    margin: 0.45rem 1rem;
    background: linear-gradient(to right, transparent, var(--hairline), transparent);
  }

  .zone-footer {
    display: flex;
    gap: 0.35rem;
    margin: 0 0.55rem 0.6rem;
    padding: 0 0.4rem;
  }
  .zone-footer .detail {
    flex: 1;
    background: var(--bg-detail);
    border: 1px solid var(--border-detail);
    padding: 0.45rem 0.4rem 0.5rem;
    text-align: center;
  }
  .zone-footer .detail h6 {
    margin: 0 0 0.25rem;
    font-family: 'Spartan', sans-serif;
    font-weight: 200;
    font-size: 0.58rem;
    letter-spacing: 1.1px;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .zone-footer .detail .value {
    font-family: 'Oswald', sans-serif;
    font-weight: 200;
    font-size: 0.85rem;
    letter-spacing: 0.7px;
  }

  /* Preview-mode highlight: amber border + warm glow makes builder
     previews visually distinct from saved library rows. Applied to the
     inner frame so the outlined-frame structure is preserved. */
  .vamp-card.preview .frame {
    border-color: var(--accent-amber);
    box-shadow: 0 0 16px rgba(217, 165, 96, 0.22);
  }
  /* Saved-mode click-to-edit affordance. Cursor + focus ring on the
     article when it's role=button. */
  .vamp-card.editable { cursor: pointer; }
  .vamp-card.editable:focus-visible {
    outline: 1px solid var(--accent-amber);
    outline-offset: 2px;
  }
  /* WIP draft indicator — subtle dimming. */
  .vamp-card.wip {
    opacity: 0.92;
  }

  /* Hover-reveal affordances — only 🗑 on StatusTemplateCard per spec
     §"Hover-reveal edit affordance · Applies to AdvantageCard only". */
  .affordance {
    position: absolute;
    background: transparent;
    border: 0;
    padding: 0;
    cursor: pointer;
    color: var(--text-muted);
    font-size: 0.85rem;
    line-height: 1;
    opacity: 0;
    transition: opacity 120ms ease-out;
    z-index: 2;
  }
  .vamp-card:hover .delete-icon { opacity: 0.6; }
  .affordance:focus-visible {
    opacity: 1;
    outline: 1px solid var(--accent-amber);
    outline-offset: 2px;
  }
  .delete-icon { bottom: 0.5rem; right: 0.5rem; }
  .delete-icon:hover { color: var(--accent-bright); }

  /* Apply action row — visible primary action below the outlined frame.
     Apply is high-frequency in the GM Screen flow; spec's hover-reveal
     pattern would harm discoverability, so we keep it persistent. */
  .action-row {
    display: flex;
    justify-content: stretch;
    margin: 0.4rem 0 0;
    padding: 0;
  }
  .action-row .apply {
    flex: 1;
    background: var(--accent);
    color: var(--text-primary);
    font-family: 'Spartan', sans-serif;
    font-weight: 400;
    font-size: 0.75rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    padding: 0.45rem 0.6rem;
    border: 1px solid var(--accent);
    cursor: pointer;
    transition: background 120ms ease;
  }
  .action-row .apply:hover:not(:disabled) {
    background: var(--accent-bright);
    border-color: var(--accent-bright);
  }
  .action-row .apply:disabled {
    background: transparent;
    border-color: var(--text-muted);
    color: var(--text-muted);
    cursor: not-allowed;
  }
  .action-row .apply:focus-visible {
    outline: 1px solid var(--accent-amber);
    outline-offset: 2px;
  }
</style>
