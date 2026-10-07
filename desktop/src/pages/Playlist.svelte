<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import { app, call, catalog, clock, formatMs, soundsById, useClock, type PlaylistItem } from '../lib/store.svelte';

  const ic = catalog.ui;
  const s = $derived(app.s!);
  const pl = $derived(s.playlist);

  let tab = $state<'mixes' | 'sounds'>(app.s!.mixes.length ? 'mixes' : 'sounds');
  let query = $state('');
  let minutes = $state(15);

  useClock(() => pl.running && pl.endsAt);

  const total = $derived(pl.items.reduce((n, i) => n + i.minutes, 0));
  const remaining = $derived(pl.endsAt ? pl.endsAt - clock.now : (pl.remainingMs ?? 0));

  function info(item: PlaylistItem) {
    if (item.kind === 'mix') {
      const mix = s.mixes.find(m => m.id === item.target);
      const first = mix && Object.keys(mix.sounds)[0];
      return { icon: catalog.icons[soundsById.get(first ?? '')?.icon ?? ''] ?? ic.mixes, name: mix?.name ?? 'Deleted mix' };
    }
    const sound = soundsById.get(item.target);
    return { icon: catalog.icons[sound?.icon ?? ''], name: sound?.label ?? item.target };
  }

  const sounds = $derived(
    catalog.categories
      .flatMap(c => c.sounds)
      .filter(x => x.label.toLowerCase().includes(query.trim().toLowerCase())),
  );
  const mixes = $derived(s.mixes.filter(m => m.name.toLowerCase().includes(query.trim().toLowerCase())));

  const add = (kind: 'sound' | 'mix', target: string) =>
    call('playlist_add', { kind, minutes: Math.max(0.1, Number(minutes) || 15), target });

  const options = (patch: { looping?: boolean; shuffle?: boolean }) =>
    call('playlist_options', { looping: patch.looping ?? pl.looping, shuffle: patch.shuffle ?? pl.shuffle });

  function hm(min: number) {
    if (min < 1) return `${Math.round(min * 60)} s`;
    const h = Math.floor(min / 60);
    const m = Math.round(min % 60);
    return h ? `${h} hr ${m} min` : `${m} min`;
  }
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>Playlist</h1>
      <p>Queue sounds and mixes, each playing for as long as you choose.</p>
    </div>
  </div>

  <div class="layout">
    <section>
      <div class="controls">
        {#if pl.running}
          <button class="btn primary" onclick={() => call('playlist_stop')}><Icon svg={ic.stop} /> Stop</button>
          <button aria-label="Previous" class="icon-btn" onclick={() => call('playlist_skip', { delta: -1 })}>
            <Icon svg={ic.prev} />
          </button>
          <button aria-label="Next" class="icon-btn" onclick={() => call('playlist_skip', { delta: 1 })}>
            <Icon svg={ic.next} />
          </button>
        {:else}
          <button class="btn primary" disabled={!pl.items.length} onclick={() => call('playlist_start', { from: null })}>
            <Icon svg={ic.play} /> Start
          </button>
        {/if}
        <div class="spacer"></div>
        <button
          aria-pressed={pl.shuffle}
          class="btn ghost small"
          class:on={pl.shuffle}
          onclick={() => options({ shuffle: !pl.shuffle })}
        >
          <Icon svg={ic.shuffle} /> Shuffle
        </button>
        <button
          aria-pressed={pl.looping}
          class="btn ghost small"
          class:on={pl.looping}
          onclick={() => options({ looping: !pl.looping })}
        >
          <Icon svg={ic.loop} /> Loop
        </button>
        <button class="btn ghost small" disabled={!pl.items.length} onclick={() => call('playlist_clear')}>Clear</button>
      </div>

      {#if pl.items.length === 0}
        <div class="empty">
          <Icon svg={ic.playlist} />
          The queue is empty. Add mixes or sounds from the list.
        </div>
      {:else}
        <ol class="queue">
          {#each pl.items as item, i (item.id)}
            {@const d = info(item)}
            {@const current = pl.current === item.id}
            <li class:current>
              <button
                aria-label="Play from {d.name}"
                class="idx"
                onclick={() => call('playlist_start', { from: item.id })}
                title="Play from here"
              >
                {#if current}<Icon svg={s.playing ? ic.volume : ic.pause} />{:else}{i + 1}{/if}
              </button>
              <span class="ico"><Icon svg={d.icon} /></span>
              <span class="name">
                {d.name}
                <span class="kind">{item.kind}</span>
                {#if current}
                  <span class="left">{formatMs(remaining)} left</span>
                {/if}
              </span>
              <label class="mins">
                <input
                  aria-label="Minutes for {d.name}"
                  class="input num"
                  min="0.1"
                  onchange={e => call('playlist_set_minutes', { id: item.id, minutes: Number(e.currentTarget.value) || item.minutes })}
                  step="1"
                  type="number"
                  value={item.minutes}
                />
                min
              </label>
              <button aria-label="Move up" class="icon-btn" disabled={i === 0} onclick={() => call('playlist_move', { id: item.id, to: i - 1 })}>
                <Icon svg={ic.up} />
              </button>
              <button
                aria-label="Move down"
                class="icon-btn"
                disabled={i === pl.items.length - 1}
                onclick={() => call('playlist_move', { id: item.id, to: i + 1 })}
              >
                <Icon svg={ic.down} />
              </button>
              <button aria-label="Remove {d.name}" class="icon-btn" onclick={() => call('playlist_remove', { id: item.id })}>
                <Icon svg={ic.close} />
              </button>
              {#if current}
                <span class="progress" style:width="{Math.min(100, 100 - (remaining / (item.minutes * 60000)) * 100)}%"></span>
              {/if}
            </li>
          {/each}
        </ol>
        <p class="total">{pl.items.length} item{pl.items.length === 1 ? '' : 's'} · {hm(total)}{pl.looping ? ' · looping' : ''}</p>
      {/if}
    </section>

    <aside class="panel">
      <div class="section-title">Add to queue</div>
      <div class="tabs">
        <button class:on={tab === 'mixes'} onclick={() => (tab = 'mixes')}>Mixes</button>
        <button class:on={tab === 'sounds'} onclick={() => (tab = 'sounds')}>Sounds</button>
      </div>
      <div class="row">
        <input bind:value={query} class="input" placeholder="Search" type="search" />
        <label class="mins">
          <input aria-label="Default minutes" bind:value={minutes} class="input num" min="0.1" type="number" />
          min
        </label>
      </div>
      <div class="pick">
        {#if tab === 'mixes'}
          {#each mixes as mix (mix.id)}
            <button onclick={() => add('mix', mix.id)}>
              <span>{mix.name}</span><Icon svg={ic.plus} />
            </button>
          {:else}
            <p class="none">{s.mixes.length ? 'No matches.' : 'No saved mixes yet.'}</p>
          {/each}
        {:else}
          {#each sounds as sound (sound.id)}
            <button onclick={() => add('sound', sound.id)}>
              <span class="lbl"><Icon svg={catalog.icons[sound.icon]} />{sound.label}</span><Icon svg={ic.plus} />
            </button>
          {:else}
            <p class="none">No matches.</p>
          {/each}
        {/if}
      </div>
    </aside>
  </div>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 300px;
    gap: 24px;
    align-items: start;
  }

  @media (width <= 820px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .controls {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    margin-bottom: 14px;

    & .spacer {
      flex: 1;
    }
  }

  .queue {
    display: flex;
    flex-direction: column;
    gap: 6px;
    list-style: none;

    & li {
      position: relative;
      display: flex;
      gap: 8px;
      align-items: center;
      padding: 8px 8px 8px 0;
      overflow: hidden;
      border: 1px solid var(--color-border);

      &.current {
        background-color: var(--color-surface);
        border-color: var(--color-selected-border);
      }
    }
  }

  .idx {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 40px;
    font-size: var(--font-xsm);
    color: var(--color-foreground-subtler);
    cursor: pointer;
    background: none;
    border: none;
    font-variant-numeric: tabular-nums;

    &:hover {
      color: var(--color-foreground);
    }
  }

  .ico {
    font-size: 16px;
    color: var(--color-foreground-subtle);
  }

  .name {
    display: flex;
    flex: 1;
    flex-wrap: wrap;
    gap: 4px 10px;
    align-items: baseline;
    min-width: 0;
    font-family: var(--font-heading);
    font-size: var(--font-sm);
    font-weight: 600;
  }

  .kind,
  .left {
    font-family: var(--font-body);
    font-size: var(--font-2xsm);
    font-weight: 400;
    color: var(--color-foreground-subtler);
    text-transform: capitalize;
  }

  .left {
    color: var(--color-foreground-subtle);
    text-transform: none;
    font-variant-numeric: tabular-nums;
  }

  .mins {
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: var(--font-xsm);
    color: var(--color-foreground-subtle);

    & .input {
      height: 32px;
    }
  }

  .progress {
    position: absolute;
    bottom: 0;
    left: 0;
    height: 2px;
    background-color: var(--color-inverted-background);
    transition: width 1s linear;
  }

  .total {
    margin-top: 12px;
    font-size: var(--font-xsm);
    color: var(--color-foreground-subtler);
  }

  .tabs {
    display: flex;
    margin-bottom: 10px;
    border: 1px solid var(--color-border-strong);

    & button {
      flex: 1;
      height: 32px;
      font-size: var(--font-xsm);
      color: var(--color-foreground-subtle);
      cursor: pointer;
      background: none;
      border: none;

      &.on {
        color: var(--color-inverted-foreground);
        background-color: var(--color-inverted-background);
      }
    }
  }

  .row {
    display: flex;
    gap: 8px;
    margin-bottom: 10px;

    & .input {
      height: 34px;
    }
  }

  .pick {
    display: flex;
    flex-direction: column;
    max-height: 420px;
    overflow-y: auto;

    & button {
      display: flex;
      gap: 8px;
      align-items: center;
      justify-content: space-between;
      min-height: 36px;
      padding: 0 10px;
      font-size: var(--font-sm);
      color: var(--color-foreground-subtle);
      text-align: left;
      cursor: pointer;
      background: none;
      border: 1px solid transparent;

      &:hover {
        color: var(--color-foreground);
        background-color: var(--color-surface-hover);
        border-color: var(--color-border-strong);
      }
    }

    & .lbl {
      display: flex;
      gap: 10px;
      align-items: center;
    }

    & .none {
      padding: 12px 10px;
      font-size: var(--font-sm);
      color: var(--color-foreground-subtler);
    }
  }
</style>
