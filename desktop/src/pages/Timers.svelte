<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import { app, call, catalog, clock, formatMs, go, setSettings, useClock } from '../lib/store.svelte';

  const ic = catalog.ui;
  const s = $derived(app.s!);

  useClock(() => s.sleep || s.timers.some(t => t.endsAt));

  const sleepPresets = [15, 30, 45, 60, 90, 120];
  const timerPresets = [5, 10, 15, 25, 45, 60];

  let sleepCustom = $state(20);
  let label = $state('');
  let h = $state(0);
  let m = $state(25);
  let sec = $state(0);

  const sleepLeft = $derived(s.sleep ? s.sleep.endsAt - clock.now : 0);
  const fading = $derived(s.sleep && sleepLeft <= s.settings.sleepFadeSecs * 1000);

  function addTimer(minutes: number) {
    if (minutes <= 0) return;
    call('timer_add', { label, minutes });
    label = '';
  }

  const remaining = (t: (typeof s.timers)[number]) => (t.endsAt ? t.endsAt - clock.now : t.remainingMs);
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>Timers</h1>
      <p>Timers keep running when the window is closed.</p>
    </div>
  </div>

  <section class="panel sleep">
    <div class="head">
      <div>
        <div class="section-title">Sleep timer</div>
        <p class="desc">Fades out over {s.settings.sleepFadeSecs}s, then stops playback.</p>
      </div>
      <button class="btn ghost small" onclick={() => go('clock')}><Icon svg={ic.moon} /> Sleep clock</button>
    </div>

    {#if s.sleep}
      <div class="readout">
        <span class="big">{formatMs(sleepLeft)}</span>
        <span class="note">{fading ? 'Fading out…' : `Stops at ${new Date(s.sleep.endsAt).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: !s.settings.clock24h })}`}</span>
      </div>
      <div class="bar"><span style:width="{(1 - sleepLeft / s.sleep.durationMs) * 100}%"></span></div>
      <div class="row">
        <button class="btn" onclick={() => call('sleep_start', { minutes: (sleepLeft + 15 * 60000) / 60000 })}>+15 min</button>
        <button class="btn" onclick={() => call('sleep_cancel')}>Cancel</button>
      </div>
    {:else}
      <div class="row wrap">
        {#each sleepPresets as p (p)}
          <button class="btn ghost" onclick={() => call('sleep_start', { minutes: p })}>{p < 60 ? `${p} min` : `${p / 60} hr`}</button>
        {/each}
        <span class="custom">
          <input aria-label="Custom minutes" bind:value={sleepCustom} class="input num" min="1" type="number" />
          <button class="btn primary" onclick={() => sleepCustom > 0 && call('sleep_start', { minutes: sleepCustom })}>Start</button>
        </span>
      </div>
    {/if}

    <label class="fade">
      <span>Fade out</span>
      <input
        class="range"
        max="300"
        min="0"
        oninput={e => setSettings({ sleepFadeSecs: Number(e.currentTarget.value) })}
        step="5"
        type="range"
        value={s.settings.sleepFadeSecs}
      />
      <span class="val">{s.settings.sleepFadeSecs}s</span>
    </label>
  </section>

  <section class="countdowns">
    <div class="section-title">Countdowns</div>
    <form
      class="panel add"
      onsubmit={e => {
        e.preventDefault();
        addTimer(h * 60 + m + sec / 60);
      }}
    >
      <input bind:value={label} class="input" maxlength="40" placeholder="Label (optional)" />
      <span class="hms">
        <input aria-label="Hours" bind:value={h} class="input num" max="23" min="0" type="number" /><span>h</span>
        <input aria-label="Minutes" bind:value={m} class="input num" max="59" min="0" type="number" /><span>m</span>
        <input aria-label="Seconds" bind:value={sec} class="input num" max="59" min="0" type="number" /><span>s</span>
      </span>
      <button class="btn primary" type="submit"><Icon svg={ic.plus} /> Add</button>
      <div class="presets">
        {#each timerPresets as p (p)}
          <button class="btn ghost small" onclick={() => addTimer(p)} type="button">{p} min</button>
        {/each}
      </div>
    </form>

    {#if s.timers.length}
      <div class="timers">
        {#each s.timers as t (t.id)}
          {@const left = remaining(t)}
          {@const done = left <= 0 && !t.endsAt}
          <div class="timer" class:done class:running={!!t.endsAt}>
            <div class="top">
              <span class="label">{t.label}</span>
              <button aria-label="Remove timer" class="icon-btn" onclick={() => call('timer_remove', { id: t.id })}>
                <Icon svg={ic.close} />
              </button>
            </div>
            <div class="time">{done ? 'Done' : formatMs(left)}</div>
            <div class="bar"><span style:width="{(1 - left / t.durationMs) * 100}%"></span></div>
            <div class="row">
              <button class="btn small" onclick={() => call('timer_toggle', { id: t.id })}>
                <Icon svg={t.endsAt ? ic.pause : ic.play} />
                {t.endsAt ? 'Pause' : done ? 'Restart' : 'Resume'}
              </button>
              <button class="btn small ghost" onclick={() => call('timer_reset', { id: t.id })}>Reset</button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </section>
</div>

<style>
  .sleep {
    margin-bottom: 28px;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: 16px;

    & .section-title {
      margin-bottom: 4px;
    }
  }

  .desc {
    font-size: var(--font-xsm);
    color: var(--color-foreground-subtle);
  }

  .readout {
    display: flex;
    gap: 16px;
    align-items: baseline;
    margin-bottom: 12px;
  }

  .big {
    font-family: var(--font-display);
    font-size: var(--font-3xlg);
    font-weight: 500;
    line-height: 1;
    font-variant-numeric: tabular-nums;
  }

  .note {
    font-size: var(--font-sm);
    color: var(--color-foreground-subtle);
  }

  .bar {
    height: 3px;
    margin-bottom: 14px;
    background-color: var(--color-surface-active);

    & span {
      display: block;
      height: 100%;
      background-color: var(--color-inverted-background);
      transition: width 1s linear;
    }
  }

  .row {
    display: flex;
    gap: 8px;
    align-items: center;

    &.wrap {
      flex-wrap: wrap;
    }
  }

  .custom {
    display: flex;
    gap: 6px;
    margin-left: auto;

    & .input {
      height: 36px;
    }
  }

  .fade {
    display: flex;
    gap: 12px;
    align-items: center;
    max-width: 360px;
    margin-top: 18px;
    font-size: var(--font-xsm);
    color: var(--color-foreground-subtle);
    white-space: nowrap;

    & .val {
      width: 40px;
      font-variant-numeric: tabular-nums;
    }
  }

  .add {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
    margin-bottom: 16px;

    & > .input {
      flex: 1;
      min-width: 160px;
    }

    & .presets {
      display: flex;
      flex-wrap: wrap;
      gap: 6px;
      width: 100%;
    }
  }

  .hms {
    display: flex;
    gap: 4px;
    align-items: center;
    font-size: var(--font-xsm);
    color: var(--color-foreground-subtle);

    & .input {
      width: 58px;
    }
  }

  .timers {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 12px;
  }

  .timer {
    padding: 14px;
    border: 1px solid var(--color-border);

    &.running {
      background-color: var(--color-surface);
      border-color: var(--color-selected-border);
    }

    &.done {
      border-color: var(--color-inverted-background);
    }

    & .top {
      display: flex;
      align-items: center;
      justify-content: space-between;
    }

    & .label {
      font-family: var(--font-heading);
      font-size: var(--font-sm);
      font-weight: 600;
    }

    & .time {
      margin: 6px 0 10px;
      font-family: var(--font-display);
      font-size: var(--font-2xlg);
      font-weight: 500;
      font-variant-numeric: tabular-nums;
    }
  }
</style>
