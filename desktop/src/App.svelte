<script lang="ts">
  import Icon from './components/Icon.svelte';
  import Drawer from './components/Drawer.svelte';
  import PlayerBar from './components/PlayerBar.svelte';
  import SaveDialog from './components/SaveDialog.svelte';
  import Mixes from './pages/Mixes.svelte';
  import Sounds from './pages/Sounds.svelte';
  import Playlist from './pages/Playlist.svelte';
  import Timers from './pages/Timers.svelte';
  import Clock from './pages/Clock.svelte';
  import SettingsPage from './pages/Settings.svelte';
  import { app, applyTheme, call, catalog, go, toast, ui, type Page } from './lib/store.svelte';

  const ic = catalog.ui;
  const titles: Record<Page, string> = {
    clock: 'Sleep Clock',
    mixes: 'Mixes',
    playlist: 'Playlist',
    settings: 'Settings',
    sounds: 'Sounds',
    timers: 'Timers',
  };
  const order: Page[] = ['mixes', 'sounds', 'playlist', 'timers', 'clock', 'settings'];

  let fullscreen = $state(false);

  $effect(() => {
    if (!app.s) return;
    const pref = app.s.settings.theme;
    applyTheme(pref);
    if (pref !== 'system') return;
    const mq = matchMedia('(prefers-color-scheme: dark)');
    const onChange = () => applyTheme('system');
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  });

  function isTyping(target: EventTarget | null) {
    const el = target as HTMLElement | null;
    return !!el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT' || el.isContentEditable);
  }

  function onKey(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey;
    if (mod && e.key.toLowerCase() === 's') {
      e.preventDefault();
      if (!app.s || Object.keys(app.s.sounds).length === 0) {
        toast('Select some sounds before saving a mix');
      } else {
        ui.saveOpen = true;
      }
    } else if (mod && e.key.toLowerCase() === 'q') {
      e.preventDefault();
      call('quit');
    } else if (mod && /^[1-6]$/.test(e.key)) {
      e.preventDefault();
      go(order[Number(e.key) - 1]);
    } else if (e.key === 'Escape') {
      if (ui.menuOpen) ui.menuOpen = false;
      else if (fullscreen) fullscreen = false;
    } else if (e.key === ' ' && !isTyping(e.target) && !(e.target instanceof HTMLButtonElement) && !ui.saveOpen) {
      e.preventDefault();
      if (app.s) call('set_playing', { playing: !app.s.playing });
    }
  }
</script>

<svelte:window onkeydown={onKey} />

{#if app.s}
  <div class="shell" class:fullscreen>
    {#if !fullscreen}
      <header class="topbar">
        <button
          aria-expanded={ui.menuOpen}
          aria-label="Open menu"
          class="icon-btn round"
          onclick={() => (ui.menuOpen = !ui.menuOpen)}
        >
          <Icon svg={ic.menu} />
        </button>
        <div class="brand">
          <span class="wordmark">Moodist</span>
          <span class="divider">/</span>
          <span class="where">{titles[ui.page]}</span>
        </div>
      </header>
    {/if}

    <main class="content">
      {#if ui.page === 'mixes'}
        <Mixes />
      {:else if ui.page === 'sounds'}
        <Sounds />
      {:else if ui.page === 'playlist'}
        <Playlist />
      {:else if ui.page === 'timers'}
        <Timers />
      {:else if ui.page === 'clock'}
        <Clock bind:fullscreen />
      {:else}
        <SettingsPage />
      {/if}
    </main>

    {#if !fullscreen}
      <PlayerBar />
    {/if}
  </div>

  <Drawer />
  {#if ui.saveOpen}
    <SaveDialog />
  {/if}
  {#if ui.toast}
    <div class="toast" role="status">{ui.toast}</div>
  {/if}
{/if}

<style>
  .shell {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    height: 100%;

    &.fullscreen {
      grid-template-rows: minmax(0, 1fr);
    }
  }

  .topbar {
    display: flex;
    align-items: center;
    gap: 14px;
    height: 64px;
    padding: 0 16px;
    border-bottom: 1px solid var(--color-border);
  }

  .brand {
    display: flex;
    align-items: baseline;
    gap: 10px;
    min-width: 0;
  }

  .wordmark {
    font-family: var(--font-display);
    font-size: var(--font-md);
    font-weight: 600;
  }

  .divider {
    color: var(--color-foreground-subtler);
  }

  .where {
    font-family: var(--font-heading);
    font-size: var(--font-sm);
    font-weight: 600;
    color: var(--color-foreground-subtle);
  }

  .content {
    overflow-y: auto;
    scrollbar-gutter: stable;
  }

  .toast {
    position: fixed;
    bottom: 96px;
    left: 50%;
    z-index: 40;
    padding: 10px 16px;
    font-size: var(--font-sm);
    color: var(--color-inverted-foreground);
    background-color: var(--color-inverted-background);
    transform: translateX(-50%);
  }
</style>
