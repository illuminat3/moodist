<script lang="ts">
  import Icon from './Icon.svelte';
  import { app, call, catalog, go, ui, type Page } from '../lib/store.svelte';

  const ic = catalog.ui;

  const sections: Array<{ items: Array<{ icon: string; key: string; label: string; page: Page }>; title: string }> = [
    {
      items: [
        { icon: ic.mixes, key: '1', label: 'Mixes', page: 'mixes' },
        { icon: ic.sounds, key: '2', label: 'Sounds', page: 'sounds' },
        { icon: ic.playlist, key: '3', label: 'Playlist', page: 'playlist' },
      ],
      title: 'Listen',
    },
    {
      items: [
        { icon: ic.timer, key: '4', label: 'Timers', page: 'timers' },
        { icon: ic.moon, key: '5', label: 'Sleep Clock', page: 'clock' },
      ],
      title: 'Rest',
    },
  ];

  const badge = (page: Page) => {
    const s = app.s;
    if (!s) return false;
    if (page === 'playlist') return s.playlist.running;
    if (page === 'timers') return !!s.sleep || s.timers.some(t => t.endsAt);
    return false;
  };
</script>

{#if ui.menuOpen}
  <button aria-label="Close menu" class="overlay" onclick={() => (ui.menuOpen = false)} tabindex="-1"></button>
{/if}

<nav aria-hidden={!ui.menuOpen} class="drawer" class:open={ui.menuOpen} inert={!ui.menuOpen}>
  <div class="head">
    <span class="wordmark">Moodist</span>
    <button aria-label="Close menu" class="icon-btn" onclick={() => (ui.menuOpen = false)}>
      <Icon svg={ic.close} />
    </button>
  </div>

  {#each sections as section (section.title)}
    <div class="section">
      <div class="title">{section.title}</div>
      {#each section.items as item (item.page)}
        <button class="item" class:current={ui.page === item.page} onclick={() => go(item.page)}>
          <span class="label"><Icon svg={item.icon} class="ico" />{item.label}</span>
          <span class="meta">
            {#if badge(item.page)}<span class="dot"></span>{/if}
            <kbd>Ctrl {item.key}</kbd>
          </span>
        </button>
      {/each}
    </div>
  {/each}

  <div class="bottom">
    <button class="item" class:current={ui.page === 'settings'} onclick={() => go('settings')}>
      <span class="label"><Icon svg={ic.settings} class="ico" />Settings</span>
      <kbd>Ctrl 6</kbd>
    </button>
    <button class="item" onclick={() => call('quit')}>
      <span class="label"><Icon svg={ic.close} class="ico" />Quit Moodist</span>
      <kbd>Ctrl Q</kbd>
    </button>
  </div>
</nav>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 20;
    cursor: default;
    background-color: var(--color-backdrop);
    border: none;
  }

  .drawer {
    position: fixed;
    top: 0;
    bottom: 0;
    left: 0;
    z-index: 21;
    display: flex;
    flex-direction: column;
    width: 280px;
    padding: 12px 8px;
    visibility: hidden;
    background-color: var(--color-surface);
    border-right: 1px solid var(--color-border-strong);
    box-shadow: var(--shadow-floating);
    transform: translateX(-100%);
    transition:
      transform 200ms ease,
      visibility 0s linear 200ms;

    &.open {
      visibility: visible;
      transform: none;
      transition: transform 200ms ease;
    }
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 8px 14px 12px;
  }

  .wordmark {
    font-family: var(--font-display);
    font-size: var(--font-md);
    font-weight: 600;
  }

  .section {
    padding: 10px 0;
    border-top: 1px solid var(--color-border);
  }

  .title {
    padding: 4px 12px 8px;
    font-family: var(--font-heading);
    font-size: var(--font-2xsm);
    font-weight: 600;
    color: var(--color-foreground-subtler);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .bottom {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding-top: 10px;
    margin-top: auto;
    border-top: 1px solid var(--color-border);
  }

  /* Matches the web app's menu item. */
  .item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    height: 40px;
    padding: 0 12px;
    margin-bottom: 4px;
    font-size: var(--font-sm);
    color: var(--color-foreground-subtle);
    text-align: left;
    cursor: pointer;
    background: transparent;
    border: 1px solid transparent;
    outline: none;
    transition: 0.2s;

    &:hover,
    &:focus-visible,
    &.current {
      color: var(--color-foreground);
      background-color: var(--color-surface-hover);
      border-color: var(--color-border-strong);
    }

    & .label {
      display: flex;
      gap: 10px;
      align-items: center;
    }

    & :global(.ico) {
      font-size: 16px;
      color: var(--color-foreground);
    }

    & .meta {
      display: flex;
      gap: 8px;
      align-items: center;
    }

    & .dot {
      width: 5px;
      height: 5px;
      background: var(--color-inverted-background);
      border-radius: 50%;
    }
  }
</style>
