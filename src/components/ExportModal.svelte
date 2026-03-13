<!--
  FICHIER : trimlab/src/components/ExportModal.svelte
  ROLE    : Modal export — format timeline + chapitres YouTube
-->
<script lang="ts">
  import {
    showExportModal,
    activeProject,
    canExport,
    showLicenceModal,
    notify,
  } from "../lib/store";
  import { get } from "svelte/store";
  import { commands } from "../lib/commands";
  import { formatDuration } from "../lib/utils";
  import { save } from "@tauri-apps/plugin-dialog";
  let format = "fcpxml";
  let loading = false;

  // ── Chapitres YouTube ─────────────────────────────────────────────────────
  let showChapters = false;
  let chaptersLoading = false;
  let chapters: Array<{
    time_edited: number;
    time_source: number;
    title: string;
  }> = [];
  let chaptersCopied = false;

  const formats = [
    {
      id: "fcpxml",
      label: "Final Cut Pro XML",
      ext: ".fcpxml",
      desc: "Final Cut Pro X, DaVinci Resolve",
    },
    {
      id: "xml",
      label: "FCP7 XML",
      ext: ".xml",
      desc: "Premiere Pro, DaVinci (File > Import)",
    },
    {
      id: "edl",
      label: "EDL (CMX3600)",
      ext: ".edl",
      desc: "Format universel : Premiere, Avid, DaVinci, FCPX",
    },
  ];

  function formatYtTime(seconds: number): string {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = Math.floor(seconds % 60);
    const mm = String(m).padStart(2, "0");
    const ss = String(s).padStart(2, "0");
    return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
  }

  function chaptersToText(chs: typeof chapters): string {
    return chs
      .map((c) => `${formatYtTime(c.time_edited)} ${c.title}`)
      .join("\n");
  }

  async function loadChapters() {
    if (!$activeProject) return;
    chaptersLoading = true;
    showChapters = true;
    try {
      chapters = await commands.generateChapters($activeProject.id);
      if (chapters.length < 2) {
        notify({
          type: "info",
          title: "Chapitres",
          message:
            "Pas assez de pauses pour générer des chapitres. Ajoute une transcription pour de meilleurs résultats.",
        });
        chapters = [];
        showChapters = false;
      }
    } catch (e) {
      notify({ type: "error", title: "Erreur chapitres", message: String(e) });
      showChapters = false;
    } finally {
      chaptersLoading = false;
    }
  }

  async function copyChapters() {
    const text = chaptersToText(chapters);
    await navigator.clipboard.writeText(text);
    chaptersCopied = true;
    setTimeout(() => (chaptersCopied = false), 2500);
  }

  async function downloadChapters() {
    const text = chaptersToText(chapters);
    const name = ($activeProject?.name ?? "chapitres").replace(
      /[^a-zA-Z0-9_-]/g,
      "_",
    );
    const path = await save({
      defaultPath: `${name}_chapitres.txt`,
      filters: [{ name: "Texte", extensions: ["txt"] }],
    }).catch(() => null);
    if (!path) return;
    await commands.saveTextFile(path, text);
    notify({
      type: "success",
      title: "Chapitres enregistrés",
      message: path.split(/[/\\]/).pop() ?? "",
    });
  }

  // ── Export timeline ───────────────────────────────────────────────────────
  async function doExport() {
    if (!$activeProject) return;
    if (!get(canExport)) {
      showExportModal.set(false);
      showLicenceModal.set(true);
      return;
    }
    const ext = formats.find((f) => f.id === format)?.ext ?? ".fcpxml";
    const defaultName =
      $activeProject.name.replace(/[^a-zA-Z0-9_-]/g, "_") + ext;
    let outputPath: string | null = null;
    try {
      outputPath = await save({
        defaultPath: defaultName,
        filters: [{ name: "Export", extensions: [ext.replace(".", "")] }],
      });
    } catch {
      return;
    }
    if (!outputPath) return;

    loading = true;
    try {
      const res = await commands.exportSegments(
        $activeProject.id,
        format,
        outputPath,
      );
      if (res.success) {
        notify({
          type: "success",
          title: "Export terminé !",
          message: `${res.segments_exported} segments → ${outputPath}`,
        });
        showExportModal.set(false);
      } else {
        notify({
          type: "error",
          title: "Erreur d'export",
          message: "Export échoué",
        });
      }
    } catch (e) {
      notify({ type: "error", title: "Erreur d'export", message: String(e) });
    } finally {
      loading = false;
    }
  }
</script>

{#if $showExportModal}
  <div class="modal-backdrop" role="presentation">
    <button
      class="backdrop-btn"
      on:click={() => showExportModal.set(false)}
      aria-label="Fermer"
    ></button>
    <div class="modal" role="dialog" aria-modal="true">
      <!-- Header -->
      <div class="modal-header">
        <div>
          <h2 class="font-display">Exporter le projet</h2>
          <p class="text-muted" style="font-size:13px; margin-top:4px;">
            {$activeProject?.name}
          </p>
        </div>
        <button
          class="btn btn-ghost btn-sm"
          on:click={() => showExportModal.set(false)}>✕</button
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
            La prévisualisation est gratuite.<br />L'export nécessite une
            licence TrimLab.
          </p>
          <button
            class="btn btn-primary"
            on:click={() => {
              showExportModal.set(false);
              showLicenceModal.set(true);
            }}
          >
            Acheter une licence
          </button>
        </div>
      {:else}
        <!-- Format -->
        <div class="section-title">Format d'export</div>
        <div class="format-list">
          {#each formats as f}
            <button
              class="format-option"
              class:selected={format === f.id}
              on:click={() => (format = f.id)}
            >
              <div class="format-radio" class:active={format === f.id}></div>
              <div>
                <div class="format-name">
                  {f.label}
                  <span class="font-mono text-muted" style="font-size:10px;"
                    >{f.ext}</span
                  >
                </div>
                <div class="format-desc">{f.desc}</div>
              </div>
            </button>
          {/each}
        </div>

        <!-- Stats -->
        {#if $activeProject?.stats}
          {@const s = $activeProject.stats}
          <div class="export-stats">
            <div class="export-stat">
              <span class="text-muted">Durée originale</span>
              <span class="font-mono"
                >{formatDuration(s.original_duration)}</span
              >
            </div>
            <div class="export-stat">
              <span class="text-muted">Durée finale</span>
              <span class="font-mono text-accent"
                >{formatDuration(s.trimmed_duration)}</span
              >
            </div>
            <div class="export-stat">
              <span class="text-muted">Silences supprimés</span>
              <span class="font-mono">{s.silences_removed}</span>
            </div>
            <div class="export-stat">
              <span class="text-muted">Fillers supprimés</span>
              <span class="font-mono">{s.fillers_removed}</span>
            </div>
          </div>
        {/if}

        <!-- ── Chapitres YouTube ── -->
        <div class="chapters-row">
          <div class="chapters-label">
            <span class="yt-icon">▶</span>
            <div>
              <div class="chapters-title">Chapitres YouTube</div>
              <div class="chapters-sub">
                Timestamps prêts à coller dans la description
              </div>
            </div>
          </div>
          {#if !showChapters}
            <button
              class="btn btn-ghost btn-sm chapters-btn"
              on:click={loadChapters}
              disabled={chaptersLoading}
            >
              {#if chaptersLoading}<span class="spinner-mini"
                ></span>{:else}Générer{/if}
            </button>
          {/if}
        </div>

        {#if showChapters && chapters.length > 0}
          <div class="chapters-panel">
            <div class="chapters-list">
              {#each chapters as ch}
                <div class="chapter-line">
                  <span class="chapter-time font-mono"
                    >{formatYtTime(ch.time_edited)}</span
                  >
                  <span class="chapter-title-text">{ch.title}</span>
                </div>
              {/each}
            </div>
            <div class="chapters-actions">
              <button class="btn btn-ghost btn-sm" on:click={downloadChapters}
                >↓ .txt</button
              >
              <button class="btn btn-primary btn-sm" on:click={copyChapters}>
                {#if chaptersCopied}✓ Copié !{:else}📋 Copier{/if}
              </button>
              <button
                class="btn btn-ghost btn-sm"
                on:click={() => {
                  showChapters = false;
                  chapters = [];
                }}
                style="margin-left:auto; opacity:0.5; font-size:11px;"
                >Masquer</button
              >
            </div>
          </div>
        {/if}

        <div class="export-hint">
          Le fichier sera importé dans DaVinci Resolve ou Premiere Pro via <strong
            >Fichier → Importer</strong
          >.
        </div>

        <div class="export-actions">
          <button
            class="btn btn-ghost"
            on:click={() => showExportModal.set(false)}>Annuler</button
          >
          <button
            class="btn btn-primary"
            on:click={doExport}
            disabled={loading}
          >
            {#if loading}<span class="animate-spin">○</span> Export en cours…{:else}Choisir
              destination et exporter{/if}
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
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
    background: var(--bg-surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-xl);
    padding: 24px;
    width: 480px;
    max-width: calc(100vw - 32px);
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: var(--shadow-lg);
    display: flex;
    flex-direction: column;
    gap: 18px;
    z-index: 1;
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

  .paywall {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    padding: 24px 0;
  }
  .paywall-icon {
    font-size: 30px;
  }
  .paywall h3 {
    font-size: 20px;
    font-weight: 700;
  }

  .section-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .format-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .format-option {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    background: var(--bg-elevated);
    border: 1.5px solid var(--border);
    border-radius: var(--radius);
    padding: 12px 14px;
    cursor: pointer;
    text-align: left;
    transition: border-color 0.12s;
  }
  .format-option:hover {
    border-color: var(--border-strong);
  }
  .format-option.selected {
    border-color: var(--border-active);
    background: var(--accent-subtle);
  }
  .format-radio {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid var(--border-strong);
    flex-shrink: 0;
    margin-top: 2px;
  }
  .format-radio.active {
    border-color: var(--accent);
    background: var(--accent);
  }
  .format-name {
    font-size: 13px;
    font-weight: 500;
  }
  .format-desc {
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 2px;
  }

  .export-stats {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 12px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .export-stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 12px;
  }
  .export-stat span:first-child {
    font-size: 10px;
  }

  /* ── Chapitres YouTube ── */
  .chapters-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .yt-icon {
    font-size: 18px;
    flex-shrink: 0;
    color: #ff0000;
  }
  .chapters-label {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .chapters-title {
    font-size: 13px;
    font-weight: 500;
  }
  .chapters-sub {
    font-size: 11px;
    color: var(--text-muted);
  }
  .chapters-btn {
    flex-shrink: 0;
  }

  .chapters-panel {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .chapters-list {
    max-height: 180px;
    overflow-y: auto;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .chapter-line {
    display: flex;
    gap: 10px;
    align-items: baseline;
    font-size: 12px;
  }
  .chapter-time {
    color: var(--accent);
    font-size: 11px;
    flex-shrink: 0;
    width: 38px;
  }
  .chapter-title-text {
    color: var(--text-primary);
  }
  .chapters-actions {
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 8px 10px;
    border-top: 1px solid var(--border);
  }

  .export-hint {
    background: rgba(184, 255, 60, 0.05);
    border: 1px solid rgba(184, 255, 60, 0.15);
    border-radius: var(--radius-sm);
    padding: 10px 14px;
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.5;
  }
  .export-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .spinner-mini {
    display: inline-block;
    width: 10px;
    height: 10px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    vertical-align: middle;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
