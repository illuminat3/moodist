<script lang="ts">
  import Icon from './Icon.svelte';
  import { app, autoName, call, catalog, soundsById, toast, ui, type Mix } from '../lib/store.svelte';

  const ic = catalog.ui;
  const editing = ui.editing?.id ? ui.editing : null;
  let name = $state(editing?.name ?? '');
  const placeholder = autoName(app.s!.sounds);
  const sounds = Object.entries(app.s!.sounds);

  function close() {
    ui.saveOpen = false;
  }

  async function save(asNew: boolean) {
    const mix = await call<Mix | null>('save_mix', {
      id: asNew ? null : (editing?.id ?? null),
      name: name.trim() || null,
    });
    if (mix) {
      toast(`Saved “${mix.name}”`);
      if (ui.editing) ui.editing = { id: mix.id, name: mix.name };
    }
    close();
  }

  function focus(node: HTMLInputElement) {
    node.focus();
    node.select();
  }
</script>

<button aria-label="Close" class="overlay" onclick={close} tabindex="-1"></button>
<div
  aria-labelledby="save-title"
  aria-modal="true"
  class="modal"
  onkeydown={e => {
    e.stopPropagation();
    if (e.key === 'Escape') close();
  }}
  role="dialog"
  tabindex="-1"
>
  <button aria-label="Close" class="close" onclick={close}><Icon svg={ic.close} /></button>
  <h2 id="save-title">{editing ? 'Update mix' : 'Save mix'}</h2>
  <p class="desc">Saved locally with each sound's volume and swell setting.</p>

  <form
    onsubmit={e => {
      e.preventDefault();
      save(false);
    }}
  >
    <label class="field">
      <span>Name</span>
      <input bind:value={name} class="input" maxlength="80" {placeholder} use:focus />
    </label>
    {#if !name.trim()}
      <p class="hint">Leave blank to use “{placeholder}”.</p>
    {/if}

    <ul class="list">
      {#each sounds as [id, s] (id)}
        <li>
          <Icon svg={catalog.icons[soundsById.get(id)?.icon ?? '']} />
          <span class="label">{soundsById.get(id)?.label ?? id}</span>
          {#if s.swell}<span class="tag">Swell</span>{/if}
          <span class="vol">{Math.round(s.volume * 100)}%</span>
        </li>
      {/each}
    </ul>

    <div class="actions">
      <button class="btn" onclick={close} type="button">Cancel</button>
      {#if editing}
        <button class="btn" onclick={() => save(true)} type="button">Save as new</button>
      {/if}
      <button class="btn primary" type="submit">{editing ? 'Update' : 'Save'}</button>
    </div>
  </form>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 30;
    background-color: var(--color-backdrop);
    border: none;
  }

  /* Modal from the web app. */
  .modal {
    position: fixed;
    top: 50%;
    left: 50%;
    z-index: 31;
    width: min(90vw, 460px);
    max-height: 85vh;
    padding: 24px 20px 20px;
    overflow-y: auto;
    background-color: var(--color-surface);
    border: 1px solid var(--color-border);
    box-shadow: var(--shadow-floating);
    transform: translate(-50%, -50%);
  }

  .close {
    position: absolute;
    top: 10px;
    right: 10px;
    font-size: 16px;
    color: var(--color-foreground-subtle);
    cursor: pointer;
    background: none;
    border: none;
  }

  h2 {
    font-family: var(--font-heading);
    font-size: var(--font-md);
    font-weight: 600;
  }

  .desc {
    margin: 6px 0 18px;
    font-size: var(--font-sm);
    color: var(--color-foreground-subtle);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: var(--font-xsm);
    color: var(--color-foreground-subtle);
  }

  .hint {
    margin-top: 6px;
    font-size: var(--font-xsm);
    color: var(--color-foreground-subtler);
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-top: 16px;
    list-style: none;

    & li {
      display: flex;
      gap: 10px;
      align-items: center;
      padding: 8px 10px;
      font-size: var(--font-sm);
      background-color: var(--color-background);
      border: 1px solid var(--color-border);
    }

    & .label {
      flex: 1;
    }

    & .tag {
      padding: 2px 6px;
      font-size: var(--font-2xsm);
      background-color: var(--color-surface-active);
    }

    & .vol {
      font-size: var(--font-xsm);
      color: var(--color-foreground-subtle);
      font-variant-numeric: tabular-nums;
    }
  }

  .actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 20px;
  }
</style>
