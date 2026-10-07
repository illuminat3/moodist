<script lang="ts">
  import Icon from './Icon.svelte';
  import { app, autoName, call, catalog, clock, formatMs, go, sameSounds, setSettings, useClock } from '../lib/store.svelte';

  const ic = catalog.ui;
  const s = $derived(app.s!);
  const count = $derived(Object.keys(s.sounds).length);
  const activeMix = $derived(s.mixes.find(m => sameSounds(m.sounds, s.sounds)));
  const title = $derived(count === 0 ? 'Nothing selected' : (activeMix?.name ?? autoName(s.sounds)));

  const pl = $derived(s.playlist);
  const plIndex = $derived(pl.running ? pl.items.findIndex(i => i.id === pl.current) : -1);

  useClock(() => (pl.running && pl.endsAt) || s.sleep);

  const plLeft = $derived(pl.endsAt ? pl.endsAt - clock.now : (pl.remainingMs ?? 0));
  const sleepLeft = $derived(s.sleep ? s.sleep.endsAt - clock.now : 0);
</script>

<footer class="bar">
  <button
    aria-label={s.playing ? 'Pause' : 'Play'}
    class="play"
    disabled={count === 0}
    onclick={() => call('set_playing', { playing: !s.playing })}
  >
    <Icon svg={s.playing ? ic.pause : ic.play} />
    <span>{s.playing ? 'Pause' : 'Play'}</span>
  </button>

  <div class="info">
    <div class="title">{title}</div>
    <div class="sub">
      {#if pl.running}
        <button class="link" onclick={() => go('playlist')}>
          Playlist {plIndex + 1}/{pl.items.length} · {formatMs(plLeft)} left
        </button>
      {:else if count > 0}
        {count} sound{count === 1 ? '' : 's'}
      {:else}
        Pick sounds or a mix to begin
      {/if}
      {#if s.sleep}
        <button class="link" onclick={() => go('timers')}>
          <Icon svg={ic.moon} /> {formatMs(sleepLeft)}
        </button>
      {/if}
    </div>
  </div>

  <div class="controls">
    {#if pl.running}
      <button aria-label="Previous in playlist" class="icon-btn" onclick={() => call('playlist_skip', { delta: -1 })}>
        <Icon svg={ic.prev} />
      </button>
      <button aria-label="Next in playlist" class="icon-btn" onclick={() => call('playlist_skip', { delta: 1 })}>
        <Icon svg={ic.next} />
      </button>
    {/if}
    <button aria-label="Random mix" class="icon-btn" onclick={() => call('shuffle_sounds')} title="Random mix">
      <Icon svg={ic.shuffle} />
    </button>
    <button
      aria-label="Clear selection"
      class="icon-btn"
      disabled={count === 0}
      onclick={() => call('replace_sounds', { play: false, sounds: {} })}
      title="Clear selection"
    >
      <Icon svg={ic.reset} />
    </button>
    <div class="volume" title="App volume">
      <Icon svg={s.settings.masterVolume === 0 ? ic.mute : ic.volume} />
      <input
        aria-label="App volume"
        class="range"
        max="1"
        min="0"
        step="0.01"
        type="range"
        value={s.settings.masterVolume}
        oninput={e => setSettings({ masterVolume: Number(e.currentTarget.value) })}
      />
    </div>
  </div>
</footer>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 16px;
    height: 76px;
    padding: 0 18px;
    background-color: var(--color-surface);
    border-top: 1px solid var(--color-border);
  }

  /* The web app's play button, a little smaller. */
  .play {
    display: flex;
    flex: none;
    gap: 6px;
    align-items: center;
    justify-content: center;
    width: 124px;
    height: 44px;
    font-family: var(--font-heading);
    font-size: var(--font-base);
    font-weight: 600;
    color: var(--color-inverted-foreground);
    cursor: pointer;
    background-color: var(--color-inverted-background);
    border: 1px solid var(--color-background);
    border-radius: 100px;
    transition:
      background-color 180ms ease,
      transform 120ms ease;

    & :global(.icon-wrap) {
      font-size: var(--font-lg);
    }

    &:hover {
      background-color: var(--color-inverted-background-hover);
    }

    &:not(:disabled):active {
      transform: scale(0.97);
    }

    &:disabled {
      cursor: not-allowed;
      opacity: 0.5;
    }

    &:focus-visible {
      outline: 2px solid var(--color-focus);
      outline-offset: 2px;
    }
  }

  .info {
    flex: 1;
    min-width: 0;
  }

  .title {
    overflow: hidden;
    font-family: var(--font-heading);
    font-size: var(--font-sm);
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub {
    display: flex;
    gap: 12px;
    align-items: center;
    margin-top: 3px;
    font-size: var(--font-xsm);
    color: var(--color-foreground-subtle);
    white-space: nowrap;
  }

  .link {
    display: inline-flex;
    gap: 4px;
    align-items: center;
    padding: 0;
    color: inherit;
    cursor: pointer;
    background: none;
    border: none;

    &:hover {
      color: var(--color-foreground);
    }
  }

  .controls {
    display: flex;
    flex: none;
    gap: 4px;
    align-items: center;
  }

  .volume {
    display: flex;
    gap: 8px;
    align-items: center;
    width: 150px;
    margin-left: 8px;
    color: var(--color-foreground-subtle);
  }

  @media (width <= 640px) {
    .volume {
      width: 90px;
    }
  }
</style>
