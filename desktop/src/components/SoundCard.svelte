<script lang="ts">
  import Icon from './Icon.svelte';
  import { app, call, catalog, setSound, type CatalogSound } from '../lib/store.svelte';

  let { sound }: { sound: CatalogSound } = $props();

  const ic = catalog.ui;
  const setting = $derived(app.s!.sounds[sound.id]);
  const selected = $derived(!!setting);
  const favorite = $derived(app.s!.favorites.includes(sound.id));

  const toggle = () => call('toggle_sound', { id: sound.id });
</script>

<!-- Same structure and styling as the web app's sound card. -->
<div
  aria-label="{sound.label} sound"
  aria-pressed={selected}
  class="sound"
  class:selected
  onclick={toggle}
  onkeydown={e => {
    if (e.target !== e.currentTarget) return;
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      e.stopPropagation();
      toggle();
    }
  }}
  role="button"
  tabindex="0"
>
  <div class="icon"><Icon svg={catalog.icons[sound.icon]} /></div>
  <div class="content">
    <div class="heading">
      <div class="label">{sound.label}</div>
      <button
        aria-label="{favorite ? 'Remove' : 'Add'} {sound.label} {favorite ? 'from' : 'to'} favorites"
        aria-pressed={favorite}
        class="fav"
        class:on={favorite}
        onclick={e => {
          e.stopPropagation();
          call('set_favorite', { favorite: !favorite, id: sound.id });
        }}
      >
        <Icon svg={favorite ? ic.heart : ic.heartOutline} />
      </button>
    </div>
    <div class="controls">
      <input
        aria-label="{sound.label} volume"
        class="range"
        disabled={!selected}
        max="1"
        min="0"
        onclick={e => e.stopPropagation()}
        oninput={e => setSound(sound.id, { swell: setting.swell, volume: Number(e.currentTarget.value) })}
        step="0.01"
        type="range"
        value={setting?.volume ?? 0.5}
      />
      {#if selected}
        <button
          aria-label="Swell {sound.label} volume"
          aria-pressed={setting.swell}
          class="swell"
          class:on={setting.swell}
          onclick={e => {
            e.stopPropagation();
            setSound(sound.id, { swell: !setting.swell, volume: setting.volume });
          }}
          title="Gently raise and lower volume"
        >
          <Icon svg={ic.swell} />
          <span>Swell</span>
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .sound {
    position: relative;
    display: grid;
    grid-template-columns: 52px minmax(0, 1fr);
    min-height: 94px;
    text-align: left;
    cursor: pointer;
    border: 1px solid var(--color-border);
    transition:
      background-color 180ms ease,
      border-color 180ms ease;
    contain: layout paint;

    &:hover {
      border-color: var(--color-border-strong);
    }

    &:focus-visible {
      outline: 2px solid var(--color-focus);
      outline-offset: 2px;
    }

    &.selected {
      background-color: var(--color-surface);
      border-color: var(--color-selected-border);

      & .icon {
        color: var(--color-inverted-foreground);
        background-color: var(--color-inverted-background);
        background-image: none;
        border-right-color: var(--color-selected-border);
      }
    }
  }

  .icon {
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: var(--font-md);
    color: var(--color-foreground-subtler);
    background-image: repeating-linear-gradient(
      135deg,
      transparent 0,
      transparent 5px,
      var(--color-border) 5px,
      var(--color-border) 6px
    );
    border-right: 1px solid var(--color-border);
    transition:
      color 180ms ease,
      background-color 180ms ease;

    .sound:hover & {
      color: var(--color-foreground-subtle);
    }
  }

  .content {
    display: flex;
    flex-direction: column;
    gap: 6px;
    justify-content: center;
    min-width: 0;
    padding: 10px 14px;
  }

  .heading {
    display: flex;
    gap: 4px;
    align-items: center;
    justify-content: space-between;
  }

  .label {
    min-width: 0;
    font-family: var(--font-heading);
    font-size: var(--font-sm);
    font-weight: 600;
    line-height: 1.3;
  }

  .fav {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    font-size: var(--font-sm);
    color: var(--color-foreground-subtler);
    cursor: pointer;
    background: transparent;
    border: none;
    opacity: 0;
    transition:
      opacity 180ms ease,
      color 180ms ease;

    .sound:hover &,
    &:focus-visible,
    &.on {
      opacity: 1;
    }

    &:hover,
    &.on {
      color: var(--color-foreground);
    }
  }

  .controls {
    display: flex;
    gap: 8px;
    align-items: center;
    min-width: 0;

    & input {
      flex: 1;
      min-width: 0;
    }
  }

  .swell {
    display: inline-flex;
    flex: none;
    gap: 4px;
    align-items: center;
    height: 24px;
    padding: 0 6px;
    font-size: var(--font-2xsm);
    line-height: 1;
    color: var(--color-foreground-subtle);
    cursor: pointer;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    transition:
      color 180ms ease,
      background-color 180ms ease,
      border-color 180ms ease;

    & :global(.icon-wrap) {
      font-size: var(--font-sm);
    }

    &:hover,
    &:focus-visible {
      color: var(--color-foreground);
      background-color: var(--color-surface-hover);
    }

    &.on {
      color: var(--color-foreground);
      background-color: var(--color-surface-active);
      border-color: var(--color-border-strong);
    }
  }
</style>
