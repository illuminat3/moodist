<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import SoundCard from '../components/SoundCard.svelte';
  import { app, catalog, soundsById, toast, ui, type CatalogCategory } from '../lib/store.svelte';

  const ic = catalog.ui;
  let query = $state('');
  let search: HTMLInputElement;

  const favorites = $derived<CatalogCategory | null>(
    app.s!.favorites.length
      ? {
          icon: '',
          id: 'favorites',
          sounds: app.s!.favorites.map(id => soundsById.get(id)!).filter(Boolean),
          title: 'Favorites',
        }
      : null,
  );

  const categories = $derived.by(() => {
    const all = favorites ? [favorites, ...catalog.categories] : catalog.categories;
    const q = query.trim().toLowerCase();
    if (!q) return all;
    return all
      .filter(c => c.id !== 'favorites')
      .map(c => ({ ...c, sounds: c.sounds.filter(s => s.label.toLowerCase().includes(q)) }))
      .filter(c => c.sounds.length);
  });

  const count = $derived(Object.keys(app.s!.sounds).length);

  function save() {
    if (count === 0) toast('Select some sounds before saving a mix');
    else ui.saveOpen = true;
  }

  function jump(id: string) {
    document.getElementById(`category-${id}`)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }
</script>

<svelte:window
  onkeydown={e => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
      e.preventDefault();
      search.focus();
    }
  }}
/>

<div class="page">
  <div class="page-head">
    <div>
      <h1>{ui.editing ? (ui.editing.id ? `Editing “${ui.editing.name}”` : 'New mix') : 'Sounds'}</h1>
      <p>
        {#if ui.editing}
          Select sounds and set their volume, then press <kbd>Ctrl S</kbd> to save.
        {:else}
          Click a sound to add it. <kbd>Ctrl S</kbd> saves the selection as a mix.
        {/if}
      </p>
    </div>
    <div class="actions">
      {#if ui.editing}
        <button class="btn ghost" onclick={() => (ui.editing = null)}>Done</button>
      {/if}
      <button class="btn primary" disabled={count === 0} onclick={save}>
        <Icon svg={ic.save} /> Save mix
      </button>
    </div>
  </div>

  <div class="toolbar">
    <label class="search">
      <Icon svg={ic.search} />
      <input bind:this={search} bind:value={query} class="input" placeholder="Search sounds" type="search" />
    </label>
    {#if !query}
      <div class="jump">
        {#each catalog.categories as c (c.id)}
          <button aria-label="Jump to {c.title}" class="icon-btn" onclick={() => jump(c.id)} title={c.title}>
            <Icon svg={catalog.icons[c.icon]} />
          </button>
        {/each}
      </div>
    {/if}
  </div>

  {#each categories as category (category.id)}
    <section class="category" id="category-{category.id}">
      <div class="badge">
        <div class="tail"></div>
        <div class="circle">
          <Icon svg={category.id === 'favorites' ? ic.heart : catalog.icons[category.icon]} />
        </div>
      </div>
      <h2>{category.title}</h2>
      <div class="grid">
        {#each category.sounds as sound (sound.id)}
          <SoundCard {sound} />
        {/each}
      </div>
    </section>
  {:else}
    <div class="empty">No sounds match “{query}”.</div>
  {/each}
</div>

<style>
  .actions {
    display: flex;
    gap: 8px;
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
  }

  .search {
    position: relative;
    flex: 1;
    max-width: 320px;

    & :global(.icon-wrap) {
      position: absolute;
      top: 50%;
      left: 12px;
      color: var(--color-foreground-subtler);
      transform: translateY(-50%);
    }

    & input {
      padding-left: 36px;
    }
  }

  .jump {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
  }

  /* Category heading from the web app. */
  .category {
    scroll-margin-top: 0;
    content-visibility: auto;
    contain-intrinsic-size: auto 600px;

    &:not(:last-child) {
      margin-bottom: 20px;
    }
  }

  .badge {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-bottom: 15px;
  }

  .tail {
    width: 1px;
    height: 60px;
    background: linear-gradient(transparent, var(--color-border-strong));
  }

  .circle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 45px;
    height: 45px;
    font-size: var(--font-md);
    background-color: var(--color-surface);
    border: 1px solid var(--color-border-strong);
    border-radius: 50%;
  }

  h2 {
    font-family: var(--font-display);
    font-size: var(--font-lg);
    font-weight: 600;
    text-align: center;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 12px;
    margin-top: 20px;
  }

  .empty {
    margin-top: 32px;
  }
</style>
