<!--
  FICHIER : trimlab/src/components/player/VideoPlayer.svelte
  ROLE    : Lecteur video/audio natif via protocole asset:// de Tauri
            Synchronise avec le store playhead pour la timeline
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { playhead, isPlaying, zoom } from "../../lib/store";
  import type { Segment } from "../../lib/store";

  export let filePath: string; // chemin local ex: C:\videos\clip.mp4
  export let segments: Segment[] = [];
  export let duration: number = 0;

  let videoEl: HTMLVideoElement;
  let containerEl: HTMLElement;
  let assetUrl = "";
  let hasVideo = false;
  let currentTime = 0;

  // FLAG : true quand c'est NOUS qui avons demandé un seek externe
  // Pendant ce temps, onTimeUpdate ne doit PAS écrire dans playhead
  let externalSeekPending = false;

  // ── Split-screen avant/après ─────────────────────────────────────────────
  export let compareMode = false;
  let rawEl: HTMLVideoElement;

  function toggleCompare() {
    compareMode = !compareMode;
    if (compareMode && rawEl && videoEl)
      rawEl.currentTime = videoEl.currentTime;
  }

  function toAssetUrl(path: string): string {
    return convertFileSrc(path);
  }

  $: if (filePath) {
    assetUrl = toAssetUrl(filePath);
  }

  // ── Sync isPlaying → videoEl / rawEl ────────────────────────────────────
  // onMount + subscribe UNE SEULE fois.
  // Pourquoi pas $: ?
  //   Un bloc $: qui lit isPlaying (même via subscribe) fait que Svelte
  //   traque isPlaying comme dépendance → le bloc se réexécute à chaque
  //   changement → unsub/resub → le nouveau subscribe fire IMMÉDIATEMENT
  //   avec la valeur courante → play() appelé juste après pause() → boucle.
  //
  // La closure capture videoEl / rawEl par RÉFÉRENCE (let dans le composant),
  // donc elle voit toujours l'élément DOM courant même après bind:this.
  onMount(() => {
    const unsub = isPlaying.subscribe((playing) => {
      if (videoEl) {
        if (playing) {
          videoEl.play().catch(() => isPlaying.set(false));
        } else {
          videoEl.pause();
        }
      }
      if (rawEl) {
        if (playing) rawEl.play().catch(() => {});
        else rawEl.pause();
      }
    });
    return unsub; // Svelte appelle le retour de onMount comme onDestroy
  });

  // Sync playhead → videoEl en pause seulement
  // (pendant la lecture c'est onTimeUpdate qui pilote)
  $: if (videoEl && !$isPlaying) {
    const diff = Math.abs(videoEl.currentTime - $playhead);
    if (diff > 0.08) {
      externalSeekPending = true;
      videoEl.currentTime = $playhead;
    }
  }

  // rawEl suit le playhead en pause
  $: if (rawEl && !$isPlaying) {
    if (Math.abs(rawEl.currentTime - $playhead) > 0.08)
      rawEl.currentTime = $playhead;
  }

  function onTimeUpdate() {
    if (externalSeekPending) return; // ignore les events pendant notre seek
    currentTime = videoEl.currentTime;
    // Pendant la lecture : c'est la SEULE source de vérité pour playhead
    playhead.set(videoEl.currentTime);
  }

  function onSeeked() {
    // Le seek demandé est terminé, on peut reprendre la sync normale
    externalSeekPending = false;
    currentTime = videoEl.currentTime;
  }

  function onEnded() {
    isPlaying.set(false);
    playhead.set(0);
  }

  function onLoadedMetadata() {
    hasVideo = videoEl.videoWidth > 0 && videoEl.videoHeight > 0;
    duration = videoEl.duration;
  }

  function onError() {
    console.error("[VideoPlayer] Erreur chargement:", assetUrl);
  }

  // Scrub sur la barre de progression
  function onProgressClick(e: MouseEvent) {
    if (!videoEl || !duration) return;
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const ratio = (e.clientX - rect.left) / rect.width;
    const t = ratio * duration;
    externalSeekPending = true;
    videoEl.currentTime = t;
    playhead.set(t);
  }

  // Sauter au segment suivant "keep"
  function nextKeepSegment() {
    const keeps = segments.filter((s) => s.seg_type === "keep");
    const next = keeps.find((s) => s.start_time > currentTime + 0.1);
    if (next) {
      externalSeekPending = true;
      videoEl.currentTime = next.start_time;
      playhead.set(next.start_time);
    }
  }

  function prevKeepSegment() {
    const keeps = segments.filter((s) => s.seg_type === "keep");
    const prev = [...keeps]
      .reverse()
      .find((s) => s.end_time < currentTime - 0.1);
    if (prev) {
      externalSeekPending = true;
      videoEl.currentTime = prev.start_time;
      playhead.set(prev.start_time);
    }
  }

  // Vitesse de lecture
  let playbackRate = 1.0;
  const rates = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];
  function cycleRate() {
    const idx = rates.indexOf(playbackRate);
    playbackRate = rates[(idx + 1) % rates.length];
    if (videoEl) videoEl.playbackRate = playbackRate;
  }

  // Volume
  let volume = 1.0;
  $: if (videoEl) videoEl.volume = volume;

  function formatTime(t: number): string {
    const m = Math.floor(t / 60);
    const s = Math.floor(t % 60);
    const ms = Math.floor((t % 1) * 10);
    return `${m}:${s.toString().padStart(2, "0")}.${ms}`;
  }

  // Segment courant sous le playhead
  $: currentSegment = segments.find(
    (s) => currentTime >= s.start_time && currentTime <= s.end_time,
  );
  $: isSilence =
    currentSegment?.seg_type === "silence" ||
    currentSegment?.seg_type === "filler";

  // Sauter les silences/cuts automatiquement si activé
  export let skipSilences = true;
  $: if (
    skipSilences &&
    !compareMode &&
    $isPlaying &&
    currentSegment &&
    (currentSegment.seg_type === "silence" ||
      currentSegment.seg_type === "filler" ||
      currentSegment.seg_type === "cut")
  ) {
    // Chercher le prochain segment "keep" après le currentSegment
    const nextKeep = segments.find(
      (s) =>
        s.seg_type === "keep" &&
        s.start_time >= currentSegment!.end_time - 0.01,
    );
    externalSeekPending = true;
    videoEl.currentTime = nextKeep
      ? nextKeep.start_time
      : currentSegment.end_time + 0.05;
  }
</script>

<div class="player" bind:this={containerEl}>
  <!-- Video/Audio element -->
  {#if assetUrl}
    <div
      class="video-container"
      class:audio-only={!hasVideo}
      class:split={compareMode}
    >
      <!-- Panneau AVANT — visible seulement en mode comparaison -->
      {#if compareMode}
        <div class="split-pane">
          <div class="split-label before">AVANT</div>
          <video
            bind:this={rawEl}
            src={assetUrl}
            preload="metadata"
            class="video-el"
            class:hidden={!hasVideo}
            muted
          >
            <track kind="captions" />
          </video>
          {#if currentSegment && currentSegment.seg_type !== "keep"}
            <div class="split-cut-overlay">
              <span
                >{currentSegment.seg_type === "filler"
                  ? "Filler"
                  : currentSegment.seg_type === "silence"
                    ? "Silence"
                    : "Coupé"}</span
              >
            </div>
          {/if}
        </div>
      {/if}

      <!-- Panneau APRÈS — toujours visible -->
      <div class="split-pane">
        {#if compareMode}<div class="split-label after">APRÈS</div>{/if}
        <video
          bind:this={videoEl}
          src={assetUrl}
          on:timeupdate={onTimeUpdate}
          on:seeked={onSeeked}
          on:ended={onEnded}
          on:loadedmetadata={onLoadedMetadata}
          on:error={onError}
          preload="metadata"
          class="video-el"
          class:hidden={!hasVideo}
        >
          <track kind="captions" />
        </video>

        {#if !hasVideo}
          <div class="audio-viz">
            <div class="audio-icon">♫</div>
            <div class="audio-filename">{filePath.split(/[/\\]/).pop()}</div>
            {#if $isPlaying}
              <div class="audio-bars">
                {#each Array(5) as _, i}
                  <div
                    class="audio-bar"
                    style="animation-delay:{i * 0.1}s"
                  ></div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}

        {#if currentSegment && isSilence && !compareMode}
          <div class="segment-badge silence">
            {currentSegment.seg_type === "filler" ? "Filler" : "Silence"}
          </div>
        {/if}
      </div>
    </div>
  {:else}
    <div class="no-file">Aucun fichier chargé</div>
  {/if}

  <!-- Progress bar cliquable -->
  <div
    class="progress-track"
    role="slider"
    tabindex="0"
    aria-valuenow={currentTime}
    aria-valuemin={0}
    aria-valuemax={duration}
    aria-label="Position de lecture"
    on:click={onProgressClick}
    on:keydown={(e) => {
      if (e.key === "ArrowRight") {
        const t = Math.min(duration, currentTime + 5);
        externalSeekPending = true;
        videoEl.currentTime = t;
        playhead.set(t);
      }
      if (e.key === "ArrowLeft") {
        const t = Math.max(0, currentTime - 5);
        externalSeekPending = true;
        videoEl.currentTime = t;
        playhead.set(t);
      }
    }}
  >
    <!-- Segments colorés en arrière-plan -->
    {#each segments as seg}
      {#if duration > 0}
        <div
          class="seg-bg {seg.seg_type}"
          style="left:{(seg.start_time / duration) *
            100}%; width:{((seg.end_time - seg.start_time) / duration) * 100}%"
        ></div>
      {/if}
    {/each}
    <!-- Playhead -->
    {#if duration > 0}
      <div
        class="progress-fill"
        style="width:{(currentTime / duration) * 100}%"
      ></div>
      <div
        class="playhead-needle"
        style="left:{(currentTime / duration) * 100}%"
      ></div>
    {/if}
  </div>

  <!-- Controls -->
  <div class="controls">
    <div class="controls-left">
      <button
        class="ctrl-btn"
        on:click={prevKeepSegment}
        title="Segment précédent">⏮</button
      >
      <button
        class="ctrl-btn play"
        on:click={() => isPlaying.update((v) => !v)}
      >
        {$isPlaying ? "⏸" : "▶"}
      </button>
      <button
        class="ctrl-btn"
        on:click={nextKeepSegment}
        title="Segment suivant">⏭</button
      >
      <span class="timecode"
        >{formatTime(currentTime)} / {formatTime(duration)}</span
      >
    </div>

    <div class="controls-center">
      <label class="skip-toggle" title="Sauter les silences automatiquement">
        <input type="checkbox" bind:checked={skipSilences} />
        <span>Skip silences</span>
      </label>
    </div>

    <div class="controls-right">
      <!-- Volume -->
      <input
        type="range"
        min="0"
        max="1"
        step="0.05"
        bind:value={volume}
        class="vol-slider"
        title="Volume"
      />
      <!-- Vitesse -->
      <button
        class="ctrl-btn rate"
        on:click={cycleRate}
        title="Vitesse de lecture"
      >
        {playbackRate}x
      </button>
    </div>
  </div>
</div>

<style>
  .player {
    display: flex;
    flex-direction: column;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    overflow: hidden;
    gap: 0;
  }

  .video-container {
    position: relative;
    background: #000;
    aspect-ratio: 16/9;
    display: flex;
    align-items: center;
    justify-content: center;
    max-height: 280px;
    overflow: hidden;
  }
  .video-container.audio-only {
    aspect-ratio: unset;
    height: 100px;
    background: var(--bg-elevated);
  }
  .video-container.split {
    max-height: 220px;
  }

  /* Split-screen */
  .split-pane {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    overflow: hidden;
  }
  .split .split-pane:first-child {
    border-right: 2px solid rgba(255, 255, 255, 0.15);
  }
  .split-label {
    position: absolute;
    top: 6px;
    left: 8px;
    z-index: 5;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.06em;
    padding: 2px 6px;
    border-radius: 3px;
  }
  .split-label.before {
    background: rgba(239, 68, 68, 0.8);
    color: #fff;
  }
  .split-label.after {
    background: rgba(34, 197, 94, 0.8);
    color: #fff;
  }
  .split-cut-overlay {
    position: absolute;
    inset: 0;
    background: rgba(239, 68, 68, 0.2);
    border: 2px solid rgba(239, 68, 68, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
  }
  .split-cut-overlay span {
    background: rgba(239, 68, 68, 0.9);
    color: #fff;
    font-size: 11px;
    font-weight: 700;
    padding: 3px 8px;
    border-radius: 4px;
    text-transform: uppercase;
  }

  .video-el {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .video-el.hidden {
    display: none;
  }

  .audio-viz {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 20px;
  }
  .audio-icon {
    font-size: 28px;
    opacity: 0.4;
  }
  .audio-filename {
    font-size: 12px;
    color: var(--text-muted);
  }
  .audio-bars {
    display: flex;
    gap: 3px;
    align-items: flex-end;
    height: 20px;
  }
  .audio-bar {
    width: 3px;
    background: var(--accent);
    border-radius: 2px;
    animation: bar-dance 0.5s ease-in-out infinite alternate;
  }
  @keyframes bar-dance {
    from {
      height: 4px;
    }
    to {
      height: 18px;
    }
  }

  .segment-badge {
    position: absolute;
    top: 8px;
    right: 8px;
    padding: 3px 8px;
    border-radius: var(--radius-sm);
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
  }
  .segment-badge.silence {
    background: rgba(255, 75, 75, 0.8);
    color: #fff;
  }

  /* Progress track */
  .progress-track {
    position: relative;
    height: 6px;
    background: var(--bg-elevated);
    cursor: pointer;
    overflow: hidden;
  }
  .progress-track:hover {
    height: 8px;
  }

  .seg-bg {
    position: absolute;
    top: 0;
    bottom: 0;
    opacity: 0.4;
  }
  .seg-bg.keep {
    background: var(--success);
  }
  .seg-bg.silence {
    background: var(--danger);
  }
  .seg-bg.filler {
    background: var(--warning);
  }
  .seg-bg.cut {
    background: var(--text-muted);
  }

  .progress-fill {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    background: rgba(255, 255, 255, 0.3);
    pointer-events: none;
  }
  .playhead-needle {
    position: absolute;
    top: -2px;
    bottom: -2px;
    width: 2px;
    background: #fff;
    transform: translateX(-50%);
    pointer-events: none;
  }

  /* Controls */
  .controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: var(--bg-surface);
    border-top: 1px solid var(--border);
    gap: 8px;
  }
  .controls-left,
  .controls-right,
  .controls-center {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .ctrl-btn {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    padding: 4px 8px;
    font-size: 13px;
    cursor: pointer;
    transition: all 0.1s;
    min-width: 28px;
  }
  .ctrl-btn:hover {
    background: var(--bg-overlay);
    border-color: var(--border-strong);
  }
  .ctrl-btn.play {
    background: var(--accent);
    color: #000;
    border-color: var(--accent);
    min-width: 36px;
    font-size: 14px;
  }
  .ctrl-btn.rate {
    font-size: 11px;
    font-family: var(--font-mono);
    min-width: 38px;
  }

  .timecode {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-secondary);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 3px 7px;
  }

  .skip-toggle {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .skip-toggle input {
    accent-color: var(--accent);
  }

  .vol-slider {
    width: 70px;
    accent-color: var(--accent);
    cursor: pointer;
  }

  .no-file {
    padding: 40px;
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
  }
</style>
