<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import { app, call, catalog, go, sameSounds, soundsById, toast, ui, type Mix } from '../lib/store.svelte';

  const ic = catalog.ui;
  const s = $derived(app.s!);
  const mixes = $derived([...s.mixes].sort((a, b) => b.createdAt - a.createdAt));
  let confirming = $state<string | null>(null);

  function newMix() {
    call('replace_sounds', { play: false, sounds: {} });
    ui.editing = { id: null, name: '' };
    go('sounds');
  }

  function edit(mix: Mix) {
    call('play_mix', { id: mix.id });
    ui.editing = { id: mix.id, name: mix.name };
    go('sounds');
  }

  function play(mix: Mix, active: boolean) {
    if (active) call('set_playing', { playing: !s.playing });
    else call('play_mix', { id: mix.id });
  }

  function queue(mix: Mix) {
    call('playlist_add', { kind: 'mix', minutes: 15, target: mix.id });
    toast(`Added “${mix.name}” to the playlist`);
  }

  function remove(mix: Mix) {
    if (confirming !== mix.id) {
      confirming = mix.id;
      return;
    }
    confirming = null;
    call('delete_mix', { id: mix.id });
  }
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>Mixes</h1>
      <p>Your saved soundscapes. Click one to play it.</p>
    </div>
    <button aria-label="New mix" class="icon-btn round" onclick={newMix} title="New mix">
      <Icon svg={ic.plus} />
    </button>
  </div>

  {#if mixes.length === 0}
    <div class="empty">
      <Icon svg={ic.mixes} />
      <div>No mixes yet. Build one from the sounds you like and press <kbd>Ctrl S</kbd>.</div>
      <button class="btn primary" onclick={newMix}><Icon svg={ic.plus} /> New mix</button>
    </div>
  {:else}
    <div class="grid">
      {#each mixes as mix (mix.id)}
        {@const active = sameSounds(mix.sounds, s.sounds)}
        {@const ids = Object.keys(mix.sounds)}
        <div class="mix" class:active class:playing={active && s.playing}>
          <button aria-label="Play {mix.name}" class="main" onclick={() => play(mix, active)}>
            <span class="art stripes">
              <span class="state"><Icon svg={active && s.playing ? ic.pause : ic.play} /></span>
            </span>
            <span class="body">
              <span class="name">{mix.name}</span>
              <span class="icons">
                {#each ids.slice(0, 6) as id (id)}
                  <span title={soundsById.get(id)?.label}>
                    <Icon svg={catalog.icons[soundsById.get(id)?.icon ?? '']} />
                  </span>
                {/each}
                {#if ids.length > 6}<span class="more">+{ids.length - 6}</span>{/if}
              </span>
              <span class="meta">
                {ids.length} sound{ids.length === 1 ? '' : 's'}
                {#if Object.values(mix.sounds).some(v => v.swell)} · swell{/if}
              </span>
            </span>
          </button>
          <div class="tools">
            <button aria-label="Add {mix.name} to playlist" class="icon-btn" onclick={() => queue(mix)} title="Add to playlist">
              <Icon svg={ic.playlist} />
            </button>
            <button aria-label="Edit {mix.name}" class="icon-btn" onclick={() => edit(mix)} title="Edit">
              <Icon svg={ic.edit} />
            </button>
            <button
              aria-label="Delete {mix.name}"
              class="icon-btn"
              class:danger={confirming === mix.id}
              onblur={() => confirming === mix.id && (confirming = null)}
              onclick={() => remove(mix)}
              title={confirming === mix.id ? 'Click again to delete' : 'Delete'}
            >
              <Icon svg={confirming === mix.id ? ic.check : ic.delete} />
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 12px;
  }

  .mix {
    position: relative;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--color-border);
    transition:
      border-color 180ms ease,
      background-color 180ms ease;

    &:hover {
      border-color: var(--color-border-strong);
    }

    &.active {
      background-color: var(--color-surface);
      border-color: var(--color-selected-border);

      & .art {
        color: var(--color-inverted-foreground);
        background-color: var(--color-inverted-background);
        background-image: none;
      }
    }
  }

  .main {
    display: grid;
    grid-template-columns: 64px minmax(0, 1fr);
    min-height: 104px;
    color: inherit;
    text-align: left;
    cursor: pointer;
    background: none;
    border: none;

    &:focus-visible {
      outline: 2px solid var(--color-focus);
      outline-offset: -2px;
    }
  }

  .art {
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 26px;
    color: var(--color-foreground-subtler);
    border-right: 1px solid var(--color-border);
    transition: color 180ms ease;

    .mix:hover & {
      color: var(--color-foreground);
    }
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 8px;
    justify-content: center;
    min-width: 0;
    padding: 12px 14px;
  }

  .name {
    overflow: hidden;
    font-family: var(--font-heading);
    font-size: var(--font-base);
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .icons {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 15px;
    color: var(--color-foreground-subtle);
  }

  .meta {
    margin-right: 100px;
  }

  .more,
  .meta {
    font-size: var(--font-2xsm);
    color: var(--color-foreground-subtler);
  }

  .tools {
    position: absolute;
    right: 6px;
    bottom: 6px;
    display: flex;
    gap: 2px;
    opacity: 0.6;
    transition: opacity 180ms ease;

    .mix:hover &,
    &:focus-within {
      opacity: 1;
    }

    & .danger {
      color: var(--color-inverted-foreground);
      background-color: var(--color-inverted-background);
    }
  }

  .empty :global(.btn) {
    margin-top: 6px;
  }
</style>
