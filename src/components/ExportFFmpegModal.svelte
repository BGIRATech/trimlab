<script lang="ts">
  import { save } from "@tauri-apps/plugin-dialog";
  import {
    activeProject,
    canExport,
    showLicenceModal,
    notify,
    showExportFFmpegModal,
  } from "../lib/store";
  import { get } from "svelte/store";
  import { commands } from "../lib/commands";
  import { formatDuration } from "../lib/utils";

  // ── Tab : video ou audio ─────────────────────────────────────────────────
  let tab: "video" | "audio" = "video";

  // ── Video ────────────────────────────────────────────────────────────────
  let mode: "copy" | "reencode" = "copy";
  let format: "mp4" | "mkv" | "mov" = "mp4";

  // ── Audio ────────────────────────────────────────────────────────────────
  let audioFormat: "mp3" | "wav" | "aac" = "mp3";
  let audioQuality: "low" | "medium" | "high" = "high";

  let loading = false;
  let progress = "";

  function setMode(v: string) {
    mode = v as "copy" | "reencode";
  }
  function setFormat(v: string) {
    format = v as "mp4" | "mkv" | "mov";
  }
  function setAudioFormat(v: string) {
    audioFormat = v as "mp3" | "wav" | "aac";
  }
  function setAudioQuality(v: string) {
    audioQuality = v as "low" | "medium" | "high";
  }

  const modes = [
    {
      id: "copy",
      label: "Stream copy",
      desc: "Ultra-rapide — recopie sans réencoder. Quelques frames peuvent être décalées aux cuts.",
      icon: "⚡",
    },
    {
      id: "reencode",
      label: "Réencodage H.264",
      desc: "Cuts précis à la frame. Plus lent mais qualité parfaite aux jonctions.",
      icon: "🎯",
    },
  ];

  const videoFormats = [
    { id: "mp4", label: "MP4", desc: "Compatible partout" },
    { id: "mkv", label: "MKV", desc: "Conteneur universel" },
    { id: "mov", label: "MOV", desc: "Apple / Final Cut" },
  ];

  const audioFormats = [
    { id: "mp3", label: "MP3", desc: "Universel, compressé" },
    { id: "wav", label: "WAV", desc: "Sans perte, gros fichier" },
    { id: "aac", label: "AAC", desc: "Qualité Apple / YouTube" },
  ];

  const audioQualities = [
    { id: "low", label: "Basse", desc: "128 kbps" },
    { id: "medium", label: "Moyenne", desc: "192 kbps" },
    { id: "high", label: "Haute", desc: "320 kbps" },
  ];

  async function doExport() {
    if (!$activeProject) return;
    // Guard runtime — canExport vérifié côté store ET ici
    if (!get(canExport)) {
      showExportFFmpegModal.set(false);
      showLicenceModal.set(true);
      return;
    }
    if (tab === "video") await doVideoExport();
    else await doAudioExport();
  }

  async function doVideoExport() {
    const ext = format;
    const defaultName =
      $activeProject!.name.replace(/[^a-zA-Z0-9_-]/g, "_") + "_autotrim." + ext;
    const outputPath = await save({
      defaultPath: defaultName,
      filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
    }).catch(() => null);
    if (!outputPath) return;

    loading = true;
    progress =
      mode === "copy"
        ? "Découpe en cours..."
        : "Réencodage en cours (peut prendre quelques minutes)...";

    try {
      const res = await commands.exportFfmpeg({
        projectId: $activeProject!.id,
        outputPath,
        mode,
        format,
      });
      if (res.success) {
        notify({
          type: "success",
          title: "Export terminé !",
          message: `${res.segmentsExported} segments • ${formatDuration(res.durationSaved)} économisés`,
        });
        showExportFFmpegModal.set(false);
      } else {
        notify({
          type: "error",
          title: "Erreur export",
          message: res.error ?? "Échec",
        });
      }
    } catch (e) {
      notify({ type: "error", title: "Erreur FFmpeg", message: String(e) });
    } finally {
      loading = false;
      progress = "";
    }
  }

  async function doAudioExport() {
    const ext = audioFormat;
    const defaultName =
      $activeProject!.name.replace(/[^a-zA-Z0-9_-]/g, "_") + "_autotrim." + ext;
    const outputPath = await save({
      defaultPath: defaultName,
      filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
    }).catch(() => null);
    if (!outputPath) return;

    loading = true;
    progress = "Extraction audio en cours...";

    try {
      const res = await commands.exportAudio({
        projectId: $activeProject!.id,
        outputPath,
        format: audioFormat,
        quality: audioQuality,
      });
      if (res.success) {
        notify({
          type: "success",
          title: "Audio exporté !",
          message: `${res.segmentsExported} segments → ${outputPath}`,
        });
        showExportFFmpegModal.set(false);
      } else {
        notify({
          type: "error",
          title: "Erreur export audio",
          message: res.error ?? "Échec",
        });
      }
    } catch (e) {
      notify({ type: "error", title: "Erreur FFmpeg", message: String(e) });
    } finally {
      loading = false;
      progress = "";
    }
  }
</script>

{#if $showExportFFmpegModal}
  <div class="backdrop" role="presentation">
    <button
      class="backdrop-btn"
      on:click={() => showExportFFmpegModal.set(false)}
      aria-label="Fermer"
    ></button>

    <div class="modal" role="dialog" aria-modal="true">
      <div class="modal-header">
        <div>
          <h2 class="font-display">Export direct</h2>
          <p class="text-muted" style="font-size:13px; margin-top:4px;">
            Génère un fichier sans passer par un NLE
          </p>
        </div>
        <button
          class="btn btn-ghost btn-sm"
          on:click={() => showExportFFmpegModal.set(false)}>✕</button
        >
      </div>

      {#if !$canExport}
        <!-- Paywall -->
        <div class="paywall">
          <div class="paywall-icon">🔒</div>
          <h3 class="font-display">Export bloqué</h3>
          <p
            class="text-secondary"
            style="font-size:13px; text-align:center; line-height:1.6;"
          >
            La prévisualisation est gratuite.<br />L'export vidéo/audio
            nécessite une licence TrimLab.
          </p>
          <button
            class="btn btn-primary"
            on:click={() => {
              showExportFFmpegModal.set(false);
              showLicenceModal.set(true);
            }}
          >
            Acheter une licence — $49
          </button>
        </div>
      {:else}
        <!-- Onglets Vidéo / Audio -->
        <div class="tabs">
          <button
            class="tab-btn"
            class:active={tab === "video"}
            on:click={() => (tab = "video")}
          >
            🎬 Vidéo
          </button>
          <button
            class="tab-btn"
            class:active={tab === "audio"}
            on:click={() => (tab = "audio")}
          >
            🎵 Audio seul
          </button>
        </div>

        {#if tab === "video"}
          <!-- Mode de coupe -->
          <div class="section-label">Mode de coupe</div>
          <div class="option-list">
            {#each modes as m}
              <button
                class="option-row"
                class:selected={mode === m.id}
                on:click={() => setMode(m.id)}
              >
                <span class="option-icon">{m.icon}</span>
                <div>
                  <div class="option-name">{m.label}</div>
                  <div class="option-desc">{m.desc}</div>
                </div>
                <div class="option-radio" class:active={mode === m.id}></div>
              </button>
            {/each}
          </div>

          <!-- Format vidéo -->
          <div class="section-label">Format de sortie</div>
          <div class="format-tabs">
            {#each videoFormats as f}
              <button
                class="format-tab"
                class:active={format === f.id}
                on:click={() => setFormat(f.id)}
              >
                <span class="format-ext">.{f.id}</span>
                <span class="format-desc">{f.desc}</span>
              </button>
            {/each}
          </div>

          <div class="info-box">
            {#if mode === "copy"}
              <span style="color:var(--accent)">⚡ Stream copy</span>
              — FFmpeg copie les données en quelques secondes. Parfait pour les fichiers
              longs.
            {:else}
              <span style="color:var(--warning)">🎯 Réencodage</span>
              — Cuts à la frame exacte. Comptez ~2-5 min par heure de vidéo selon
              votre CPU.
            {/if}
          </div>
        {:else}
          <!-- Format audio -->
          <div class="section-label">Format audio</div>
          <div class="format-tabs">
            {#each audioFormats as f}
              <button
                class="format-tab"
                class:active={audioFormat === f.id}
                on:click={() => setAudioFormat(f.id)}
              >
                <span class="format-ext">.{f.id}</span>
                <span class="format-desc">{f.desc}</span>
              </button>
            {/each}
          </div>

          <!-- Qualité -->
          {#if audioFormat !== "wav"}
            <div class="section-label">Qualité</div>
            <div class="quality-row">
              {#each audioQualities as q}
                <button
                  class="quality-btn"
                  class:active={audioQuality === q.id}
                  on:click={() => setAudioQuality(q.id)}
                >
                  <span class="quality-label">{q.label}</span>
                  <span class="quality-desc">{q.desc}</span>
                </button>
              {/each}
            </div>
          {/if}

          <div class="info-box">
            <span style="color:var(--accent)">🎵 Audio seul</span>
            — Extrait uniquement la piste audio avec les silences coupés.
            {#if audioFormat === "wav"}Aucune compression, qualité maximale.
            {:else if audioFormat === "mp3"}Compatible avec tous les lecteurs et
              plateformes.
            {:else}Idéal pour YouTube, Apple Podcasts et streaming.{/if}
          </div>
        {/if}

        <!-- Actions -->
        <div class="actions">
          <button
            class="btn btn-ghost"
            on:click={() => showExportFFmpegModal.set(false)}>Annuler</button
          >
          <button
            class="btn btn-primary"
            on:click={doExport}
            disabled={loading}
          >
            {#if loading}
              <span class="spinner"></span> {progress}
            {:else}
              Choisir destination et exporter
            {/if}
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(8px);
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .backdrop-btn {
    position: absolute;
    inset: 0;
    background: transparent;
    border: none;
    cursor: pointer;
  }
  .modal {
    position: relative;
    z-index: 1;
    background: var(--bg-surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-xl);
    padding: 28px;
    width: 500px;
    max-width: calc(100vw - 32px);
    box-shadow: var(--shadow-lg);
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .modal-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
  }
  .modal-header h2 {
    font-size: 18px;
    font-weight: 700;
  }

  /* Tabs */
  .tabs {
    display: flex;
    gap: 4px;
    background: var(--bg-elevated);
    border-radius: var(--radius);
    padding: 3px;
  }
  .tab-btn {
    flex: 1;
    padding: 7px 12px;
    border-radius: calc(var(--radius) - 2px);
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
    font-family: var(--font-body);
  }
  .tab-btn.active {
    background: var(--bg-surface);
    color: var(--text-primary);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.3);
  }
  .tab-btn:hover:not(.active) {
    color: var(--text-primary);
  }

  .section-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
  }
  .option-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .option-row {
    display: flex;
    align-items: center;
    gap: 12px;
    background: var(--bg-elevated);
    border: 1.5px solid var(--border);
    border-radius: var(--radius);
    padding: 12px 14px;
    cursor: pointer;
    text-align: left;
    transition: border-color 0.12s;
  }
  .option-row:hover {
    border-color: var(--border-strong);
  }
  .option-row.selected {
    border-color: var(--border-active);
    background: var(--accent-subtle);
  }
  .option-icon {
    font-size: 20px;
    flex-shrink: 0;
  }
  .option-name {
    font-size: 13px;
    font-weight: 600;
  }
  .option-desc {
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 2px;
    line-height: 1.4;
  }
  .option-radio {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid var(--border-strong);
    flex-shrink: 0;
    margin-left: auto;
  }
  .option-radio.active {
    border-color: var(--accent);
    background: var(--accent);
  }

  .format-tabs {
    display: flex;
    gap: 8px;
  }
  .format-tab {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    background: var(--bg-elevated);
    border: 1.5px solid var(--border);
    border-radius: var(--radius);
    padding: 10px 8px;
    cursor: pointer;
    transition: border-color 0.12s;
  }
  .format-tab:hover {
    border-color: var(--border-strong);
  }
  .format-tab.active {
    border-color: var(--border-active);
    background: var(--accent-subtle);
  }
  .format-ext {
    font-family: var(--font-mono);
    font-size: 14px;
    font-weight: 700;
    color: var(--accent);
  }
  .format-desc {
    font-size: 10px;
    color: var(--text-muted);
  }

  /* Qualité audio */
  .quality-row {
    display: flex;
    gap: 8px;
  }
  .quality-btn {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    background: var(--bg-elevated);
    border: 1.5px solid var(--border);
    border-radius: var(--radius);
    padding: 8px 6px;
    cursor: pointer;
    transition: border-color 0.12s;
    font-family: var(--font-body);
  }
  .quality-btn:hover {
    border-color: var(--border-strong);
  }
  .quality-btn.active {
    border-color: var(--border-active);
    background: var(--accent-subtle);
  }
  .quality-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .quality-desc {
    font-size: 10px;
    color: var(--text-muted);
  }

  .info-box {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 10px 14px;
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.5;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .spinner {
    display: inline-block;
    width: 12px;
    height: 12px;
    border: 2px solid rgba(0, 0, 0, 0.3);
    border-top-color: currentColor;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .paywall {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 40px 24px;
    text-align: center;
  }
  .paywall-icon {
    font-size: 40px;
  }
  .paywall h3 {
    font-size: 20px;
    margin: 0;
  }
  .paywall p {
    margin: 0;
    max-width: 280px;
    color: var(--text-secondary);
  }
</style>
