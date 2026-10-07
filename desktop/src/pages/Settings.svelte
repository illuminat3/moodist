<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import { app, call, catalog, setSettings } from '../lib/store.svelte';

  const ic = catalog.ui;
  const st = $derived(app.s!.settings);

  let devices = $state<Array<{ description: string; name: string }>>([]);
  let loading = $state(false);

  async function refresh() {
    loading = true;
    devices = await call('list_devices');
    loading = false;
  }
  refresh();

  const missing = $derived(st.device && !loading && !devices.some(d => d.name === st.device));
  const pct = (v: number) => `${Math.round(v * 100)}%`;

  const shortcuts = [
    ['Space', 'Play / pause'],
    ['Ctrl S', 'Save current sounds as a mix'],
    ['Ctrl F', 'Search sounds'],
    ['Ctrl 1–6', 'Switch page'],
    ['Ctrl Q', 'Quit completely'],
    ['Esc', 'Close menu / exit fullscreen'],
  ];
</script>

<div class="page narrow">
  <div class="page-head">
    <div>
      <h1>Settings</h1>
      <p>Saved to ~/.config/moodist.</p>
    </div>
  </div>

  <section>
    <div class="section-title">Audio</div>
    <div class="panel rows">
      <div class="row">
        <div class="label">
          <span>Output device</span>
          <small>{missing ? 'Saved device is unavailable; using the system default.' : 'Where Moodist plays its sound.'}</small>
        </div>
        <div class="ctrl">
          <select
            aria-label="Output device"
            class="input"
            onchange={e => setSettings({ device: e.currentTarget.value || null })}
            value={st.device ?? ''}
          >
            <option value="">System default</option>
            {#each devices as d (d.name)}
              <option value={d.name}>{d.description}</option>
            {/each}
            {#if missing}
              <option value={st.device}>{st.device} (unavailable)</option>
            {/if}
          </select>
          <button aria-label="Refresh devices" class="icon-btn" disabled={loading} onclick={refresh} title="Refresh">
            <Icon svg={ic.reset} />
          </button>
        </div>
      </div>

      <div class="row">
        <div class="label"><span>App volume</span><small>Applies to every sound.</small></div>
        <div class="ctrl slider">
          <input
            class="range"
            max="1"
            min="0"
            oninput={e => setSettings({ masterVolume: Number(e.currentTarget.value) })}
            step="0.01"
            type="range"
            value={st.masterVolume}
          />
          <span class="val">{pct(st.masterVolume)}</span>
        </div>
      </div>

      <div class="row">
        <div class="label"><span>Alarm volume</span><small>Countdown timers chime at this volume.</small></div>
        <div class="ctrl slider">
          <input
            class="range"
            max="1"
            min="0"
            oninput={e => setSettings({ alarmVolume: Number(e.currentTarget.value) })}
            step="0.01"
            type="range"
            value={st.alarmVolume}
          />
          <span class="val">{pct(st.alarmVolume)}</span>
          <button class="btn small ghost" onclick={() => call('test_alarm')}>Test</button>
        </div>
      </div>
    </div>
  </section>

  <section>
    <div class="section-title">Appearance</div>
    <div class="panel rows">
      <div class="row">
        <div class="label"><span>Theme</span></div>
        <div class="seg">
          {#each ['system', 'dark', 'light'] as const as t (t)}
            <button class:on={st.theme === t} onclick={() => setSettings({ theme: t })}>{t}</button>
          {/each}
        </div>
      </div>
    </div>
  </section>

  <section>
    <div class="section-title">Behaviour</div>
    <div class="panel rows">
      <label class="row">
        <div class="label">
          <span>Keep running when closed</span>
          <small>Closing the window keeps sounds and timers going from the tray. Quit from the tray or with Ctrl Q.</small>
        </div>
        <input
          checked={st.runInBackground}
          class="switch"
          onchange={e => setSettings({ runInBackground: e.currentTarget.checked })}
          type="checkbox"
        />
      </label>
      <div class="row">
        <div class="label"><span>Quit Moodist</span><small>Stops all sounds and exits completely.</small></div>
        <button class="btn" onclick={() => call('quit')}>Quit</button>
      </div>
    </div>
  </section>

  <section>
    <div class="section-title">Keyboard</div>
    <div class="panel keys">
      {#each shortcuts as [k, d] (k)}
        <div><kbd>{k}</kbd><span>{d}</span></div>
      {/each}
    </div>
  </section>
</div>

<style>
  .narrow {
    max-width: 760px;
  }

  section {
    margin-bottom: 28px;
  }

  .rows {
    display: flex;
    flex-direction: column;
    padding: 0;
  }

  .row {
    display: flex;
    gap: 24px;
    align-items: center;
    justify-content: space-between;
    padding: 16px 18px;

    &:not(:last-child) {
      border-bottom: 1px solid var(--color-border);
    }
  }

  label.row {
    cursor: pointer;
  }

  .label {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
    font-size: var(--font-sm);

    & small {
      font-size: var(--font-xsm);
      color: var(--color-foreground-subtle);
    }
  }

  .ctrl {
    display: flex;
    flex: none;
    gap: 6px;
    align-items: center;
    width: 320px;

    &.slider {
      gap: 12px;
    }
  }

  .val {
    width: 40px;
    font-size: var(--font-xsm);
    color: var(--color-foreground-subtle);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .seg {
    display: flex;
    border: 1px solid var(--color-border-strong);

    & button {
      height: 32px;
      padding: 0 14px;
      font-size: var(--font-xsm);
      color: var(--color-foreground-subtle);
      text-transform: capitalize;
      cursor: pointer;
      background: none;
      border: none;

      &.on {
        color: var(--color-inverted-foreground);
        background-color: var(--color-inverted-background);
      }
    }
  }

  .keys {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 10px 20px;

    & div {
      display: flex;
      gap: 12px;
      align-items: center;
      font-size: var(--font-sm);
      color: var(--color-foreground-subtle);
    }

    & kbd {
      min-width: 70px;
    }
  }

  @media (width <= 640px) {
    .ctrl {
      width: 200px;
    }
  }
</style>
