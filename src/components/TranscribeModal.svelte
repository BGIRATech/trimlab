<!--
  FICHIER : src/components/TranscribeModal.svelte
  ROLE    : Transcription Whisper — projet unique ou batch séquentiel
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    activeProject,
    projects,
    notify,
    showTranscribeModal,
  } from "../lib/store";
  import { commands } from "../lib/commands";
  // [FIX BUG G] Import de segmentsHistory (source de vérité) au lieu du store 'segments' mort
  import { segmentsHistory } from "../lib/history";

  // ── Config ───────────────────────────────────────────────────
  let language = "fr";
  let detectFillers = true;
  let detectRepeats = true;
  let repeatWindow = 40;
  let repeatThreshold = 0.5;
  let selectedModel = "base";

  // ── Sélection projets (mode batch) ───────────────────────────
  let selectedIds = new Set<string>();
  let modalWasOpen = false;

  // [FIX BUG 6] Ne réinitialiser selectedIds que à la première ouverture
  $: if ($showTranscribeModal && !modalWasOpen) {
    selectedIds = $activeProject ? new Set([$activeProject.id]) : new Set();
    modalWasOpen = true;
  }
  $: if (!$showTranscribeModal) {
    modalWasOpen = false;
  }

  function toggleProject(id: string) {
    const s = new Set(selectedIds);
    s.has(id) ? s.delete(id) : s.add(id);
    selectedIds = s;
  }
  function selectAll() {
    selectedIds = new Set($projects.map((p: any) => p.id));
  }
  function selectNone() {
    selectedIds = new Set();
  }

  $: isBatch = selectedIds.size > 1;

  // ── État UI ──────────────────────────────────────────────────
  type Phase = "config" | "running" | "done";
  let phase: Phase = "config";

  // Mode single
  let isDownloading = false;
  let downloadProgress = 0;
  // [FIX BUG 1] singleStep maintenant mis à jour via events Tauri
  let singleStep:
    | "downloading"
    | "extracting"
    | "transcribing"
    | "detecting"
    | "done" = "downloading";
  let singleResult: null | {
    fillersFound: number;
    repeatsFound: number;
    wordsCount: number;
    language: string;
  } = null;

  // Mode batch
  type JobStep =
    | "queued"
    | "loading"
    | "extracting"
    | "transcribing"
    | "detecting"
    | "done"
    | "error";
  interface JobState {
    projectId: string;
    projectName: string;
    step: JobStep;
    wordsCount: number;
    fillers: number;
    repeats: number;
    error?: string;
  }
  let batchJobs: JobState[] = [];

  // ── Listeners ─────────────────────────────────────────────────
  let unlistenDownload: UnlistenFn | null = null;
  let unlistenBatch: UnlistenFn | null = null;
  let unlistenSingle: UnlistenFn | null = null;

  onMount(async () => {
    unlistenDownload = await listen<number>(
      "whisper-download-progress",
      (e) => {
        downloadProgress = Math.round(e.payload);
      },
    );

    // [FIX BUG 1] Écouter les events de progression du mode single
    unlistenSingle = await listen<string>("whisper-single-progress", (e) => {
      singleStep = e.payload as any;
    });

    unlistenBatch = await listen<{
      projectId: string;
      step: string;
      wordsCount: number;
      fillersFound: number;
      repeatsFound: number;
      error?: string;
    }>("whisper-batch-progress", (e) => {
      const p = e.payload;
      batchJobs = batchJobs.map((j) =>
        j.projectId !== p.projectId
          ? j
          : {
              ...j,
              step: p.step as JobStep,
              wordsCount: p.wordsCount,
              fillers: p.fillersFound,
              repeats: p.repeatsFound,
              error: p.error,
            },
      );

      if (batchJobs.every((j) => j.step === "done" || j.step === "error")) {
        phase = "done";
        const ok = batchJobs.filter((j) => j.step === "done").length;
        const err = batchJobs.filter((j) => j.step === "error").length;
        if ($activeProject) {
          commands
            .listSegments($activeProject.id)
            // [FIX BUG G] segmentsHistory.init() pour que Timeline/Waveform/VideoPlayer se mettent à jour
            .then((segs) => segmentsHistory.init(segs));
        }
        notify({
          type: err > 0 ? "warning" : "success",
          title: "Transcription terminée",
          message: `${ok} projet(s) traité(s)${err > 0 ? ` · ${err} erreur(s)` : ""}`,
        });
      }
    });
  });

  onDestroy(() => {
    unlistenDownload?.();
    unlistenBatch?.();
    unlistenSingle?.();
  });

  // ── Lancement ─────────────────────────────────────────────────
  async function doTranscribe() {
    phase = "running";
    isDownloading = true;
    downloadProgress = 0;
    singleResult = null;
    singleStep = "downloading";

    let modelPath: string;
    try {
      modelPath = await commands.ensureWhisperModel(selectedModel);
    } catch (e) {
      notify({
        type: "error",
        title: "Modèle indisponible",
        message: String(e),
      });
      phase = "config";
      return;
    }
    isDownloading = false;

    const selectedProjects = $projects.filter(
      (p: any) => selectedIds.has(p.id) && p.files.length > 0,
    );

    if (selectedProjects.length === 0) {
      notify({
        type: "warning",
        title: "Aucun projet valide",
        message: "Les projets sélectionnés n'ont pas de fichier média.",
      });
      phase = "config";
      return;
    }

    // ── Batch ─────────────────────────────────────────────────
    if (isBatch) {
      batchJobs = selectedProjects.map((p: any) => ({
        projectId: p.id,
        projectName: p.name,
        step: "queued" as JobStep,
        wordsCount: 0,
        fillers: 0,
        repeats: 0,
      }));

      try {
        await commands.transcribeBatch(
          batchJobs.map((j) => {
            // [UX #15] Passer les fillers du projet concerné
            const proj = $projects.find((p: any) => p.id === j.projectId);
            return {
              projectId: j.projectId,
              modelPath,
              language,
              detectFillers,
              fillerWords: proj?.settings?.filler_words ?? [],
              detectRepeats,
              repeatWindow,
              repeatThreshold,
            };
          }),
        );
      } catch (e) {
        notify({ type: "error", title: "Erreur batch", message: String(e) });
        phase = "config";
      }
      return;
    }

    // ── Single ────────────────────────────────────────────────
    // [FIX BUG 1] singleStep est mis à jour via l'event "whisper-single-progress"
    try {
      // [UX #15] Passer les mots fillers configurés dans les settings du projet
      const fillerWords = selectedProjects[0].settings?.filler_words ?? [];
      const res = await commands.transcribeAndDetect({
        projectId: selectedProjects[0].id,
        modelPath,
        language,
        detectFillers,
        fillerWords,
        detectRepeats,
        repeatWindow,
        repeatThreshold,
      });

      if (res.success) {
        singleResult = {
          fillersFound: res.fillersFound,
          repeatsFound: res.repeatsFound,
          wordsCount: res.wordsCount,
          language: res.language,
        };
        const segs = await commands.listSegments(selectedProjects[0].id);
        // [FIX BUG G] segmentsHistory.init() pour que Timeline/Waveform/VideoPlayer se mettent à jour
        segmentsHistory.init(segs);
        notify({
          type: "success",
          title: "Transcription terminée",
          message: `${res.wordsCount} mots • ${res.fillersFound} fillers`,
        });
      } else {
        // [FIX BUG 3] Afficher l'erreur clairement même si singleResult = null
        notify({
          type: "error",
          title: "Erreur Whisper",
          message: res.error ?? "Échec",
        });
      }
    } catch (e: any) {
      notify({ type: "error", title: "Erreur", message: String(e) });
    }
    phase = "done";
  }

  function close() {
    if (phase === "running") return;
    showTranscribeModal.set(false);
    phase = "config";
    singleResult = null;
    batchJobs = [];
  }

  // ── Helpers UI ────────────────────────────────────────────────
  const stepLabel: Record<JobStep, string> = {
    queued: "En attente…",
    loading: "Chargement modèle…",
    extracting: "Extraction audio…",
    transcribing: "Transcription Whisper…",
    detecting: "Détection fillers…",
    done: "✓ Terminé",
    error: "✗ Erreur",
  };

  const singleStepLabels: Record<string, string> = {
    downloading: "Téléchargement du modèle…",
    extracting: "Extraction audio 16 kHz…",
    transcribing: "Transcription Whisper…",
    detecting: "Détection fillers & répétitions…",
    done: "Terminé !",
  };

  // [FIX BUG 4] Ajout du modèle "large"
  const models = [
    {
      id: "tiny",
      label: "Tiny",
      size: "75 MB",
      speed: "⚡⚡⚡",
      desc: "Rapide, moins précis",
    },
    {
      id: "base",
      label: "Base",
      size: "142 MB",
      speed: "⚡⚡",
      desc: "Bon équilibre",
    },
    {
      id: "small",
      label: "Small",
      size: "466 MB",
      speed: "⚡",
      desc: "Précis",
    },
    {
      id: "medium",
      label: "Medium",
      size: "1.5 GB",
      speed: "🐌",
      desc: "Très précis",
    },
    {
      id: "large",
      label: "Large",
      size: "2.9 GB",
      speed: "🐌🐌",
      desc: "Meilleure qualité",
    },
  ];

  const languages = [
    { id: "fr", label: "🇫🇷 FR" },
    { id: "en", label: "🇬🇧 EN" },
    { id: "es", label: "🇪🇸 ES" },
    { id: "de", label: "🇩🇪 DE" },
    { id: "it", label: "🇮🇹 IT" },
    { id: "pt", label: "🇵🇹 PT" },
    { id: "auto", label: "🌐 Auto" },
  ];
</script>

{#if $showTranscribeModal}
  <div class="backdrop" role="presentation">
    <button class="backdrop-btn" on:click={close} aria-label="Fermer"></button>

    <div class="modal" role="dialog" aria-modal="true">
      <!-- Header -->
      <div class="modal-header">
        <div>
          <h2 class="font-display">Transcription & Détection</h2>
          <p class="text-muted" style="font-size:13px; margin-top:4px;">
            Whisper local · 100% privé · hors-ligne
            {#if isBatch}<span class="badge-batch"
                >⚡ {selectedIds.size} clips</span
              >{/if}
          </p>
        </div>
        <button
          class="btn btn-ghost btn-sm"
          on:click={close}
          disabled={phase === "running"}>✕</button
        >
      </div>

      <!-- ══ PHASE DONE ══ -->
      {#if phase === "done"}
        {#if isBatch}
          <div class="results-list">
            {#each batchJobs as j}
              <div class="result-row" class:result-error={j.step === "error"}>
                <div class="result-name">{j.projectName}</div>
                {#if j.step === "error"}
                  <span class="result-err-msg"
                    >{j.error ?? "Erreur inconnue"}</span
                  >
                {:else}
                  <div class="result-stats">
                    <span><b>{j.wordsCount.toLocaleString()}</b> mots</span>
                    <span class="warn"><b>{j.fillers}</b> fillers</span>
                    <span class="danger"><b>{j.repeats}</b> rép.</span>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {:else if singleResult}
          <!-- [FIX BUG 5] Afficher la langue détectée -->
          {#if singleResult.language && singleResult.language !== language}
            <div class="lang-detected">
              🌐 Langue détectée : <b>{singleResult.language.toUpperCase()}</b>
            </div>
          {/if}
          <div class="results">
            <div class="result-stat">
              <span class="stat-value"
                >{singleResult.wordsCount.toLocaleString()}</span
              >
              <span class="stat-label">mots transcrits</span>
            </div>
            <div class="result-stat accent-warning">
              <span class="stat-value">{singleResult.fillersFound}</span>
              <span class="stat-label">fillers détectés</span>
            </div>
            <div class="result-stat accent-danger">
              <span class="stat-value">{singleResult.repeatsFound}</span>
              <span class="stat-label">répétitions</span>
            </div>
          </div>
        {:else}
          <!-- [FIX BUG 3] Phase done sans résultat = erreur -->
          <div class="empty-done">
            <span
              >La transcription a échoué. Vérifiez la console pour les détails.</span
            >
          </div>
        {/if}
        <div class="actions">
          <button class="btn btn-primary" on:click={close}
            >Voir la timeline</button
          >
        </div>

        <!-- ══ PHASE RUNNING ══ -->
      {:else if phase === "running"}
        {#if isDownloading}
          <div class="loading-state">
            <div class="big-spinner"></div>
            <p class="step-label">Téléchargement du modèle…</p>
            <div class="download-progress">
              <div class="progress-bar" style="width:100%">
                <div
                  class="progress-bar-fill"
                  style="width:{downloadProgress}%"
                ></div>
              </div>
              <span class="font-mono text-muted">{downloadProgress}%</span>
            </div>
          </div>
        {:else if isBatch}
          <div class="batch-jobs">
            {#each batchJobs as j}
              <div class="job-row">
                <div class="job-header">
                  <span class="job-name">{j.projectName}</span>
                  <span
                    class="job-step"
                    class:step-done={j.step === "done"}
                    class:step-error={j.step === "error"}
                  >
                    {stepLabel[j.step]}
                  </span>
                </div>
                <div class="job-bar">
                  <div class="job-fill step-{j.step}"></div>
                </div>
              </div>
            {/each}
          </div>
        {:else}
          <div class="loading-state">
            <div class="big-spinner"></div>
            <p class="step-label">{singleStepLabels[singleStep] ?? ""}</p>
            <!-- Barre de progression visuelle par étape -->
            <div class="step-indicators">
              {#each ["extracting", "transcribing", "detecting"] as s}
                <div
                  class="step-dot"
                  class:active={singleStep === s}
                  class:done={(["transcribing", "detecting", "done"].includes(
                    singleStep,
                  ) &&
                    s === "extracting") ||
                    (singleStep === "detecting" && s === "transcribing") ||
                    singleStep === "done"}
                ></div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- ══ PHASE CONFIG ══ -->
      {:else}
        <!-- Sélection projets -->
        <div class="section-label">
          Projets à traiter
          <div class="sel-btns">
            <button class="micro-btn" on:click={selectAll}>Tous</button>
            <button class="micro-btn" on:click={selectNone}>Aucun</button>
          </div>
        </div>
        <div class="projects-list">
          {#each $projects as p}
            {@const hasFile = p.files.length > 0}
            <button
              class="proj-row"
              class:selected={selectedIds.has(p.id)}
              class:disabled={!hasFile}
              disabled={!hasFile}
              on:click={() => hasFile && toggleProject(p.id)}
            >
              <span class="proj-check"
                >{selectedIds.has(p.id) ? "☑" : "☐"}</span
              >
              <span class="proj-name">{p.name}</span>
              {#if !hasFile}
                <span class="proj-tag muted">Pas de fichier</span>
              {:else}
                <span class="proj-tag">{p.files[0].media_type ?? "vidéo"}</span>
              {/if}
            </button>
          {/each}
        </div>

        <!-- Modèle — [FIX BUG 4] grille 5 modèles -->
        <div class="section-label">Modèle IA (téléchargement auto)</div>
        <div class="model-grid">
          {#each models as m}
            <button
              class="model-card"
              class:selected={selectedModel === m.id}
              on:click={() => (selectedModel = m.id)}
            >
              <div class="model-name">{m.label}</div>
              <div class="model-size">{m.size}</div>
              <div class="model-meta">{m.speed}</div>
            </button>
          {/each}
        </div>
        <p class="hint">
          {models.find((m) => m.id === selectedModel)?.desc} · ~{models.find(
            (m) => m.id === selectedModel,
          )?.size}
        </p>

        <!-- Langue — [FIX BUG 4] ajout IT, PT -->
        <div class="section-label">Langue</div>
        <div class="lang-row">
          {#each languages as l}
            <button
              class="lang-btn"
              class:active={language === l.id}
              on:click={() => (language = l.id)}>{l.label}</button
            >
          {/each}
        </div>

        <!-- Détection -->
        <div class="section-label">Détection automatique</div>
        <div class="toggles">
          <label class="toggle-row">
            <input type="checkbox" bind:checked={detectFillers} />
            <div>
              <div class="toggle-name">
                🔇 Fillers & hésitations
                <span class="badge-experimental">Expérimental</span>
              </div>
              <div class="toggle-desc">
                Euh, hm, "du coup", "en fait"… — vérifiez avant d'exporter
              </div>
            </div>
          </label>
          <label class="toggle-row">
            <input type="checkbox" bind:checked={detectRepeats} />
            <div>
              <div class="toggle-name">
                🔁 Répétitions
                <span class="badge-experimental">Expérimental</span>
              </div>
              <div class="toggle-desc">
                Phrases répétées dans une fenêtre de temps
              </div>
            </div>
          </label>
        </div>

        {#if detectRepeats}
          <div class="slider-row">
            <label for="slider-window">
              <span>Fenêtre de comparaison</span>
              <span class="slider-value">{repeatWindow}s</span>
            </label>
            <input
              id="slider-window"
              type="range"
              min="10"
              max="120"
              step="5"
              bind:value={repeatWindow}
            />
          </div>
          <div class="slider-row">
            <label for="slider-threshold">
              <span>Seuil de similarité</span>
              <span class="slider-value"
                >{Math.round(repeatThreshold * 100)}%</span
              >
            </label>
            <input
              id="slider-threshold"
              type="range"
              min="0.5"
              max="0.95"
              step="0.05"
              bind:value={repeatThreshold}
            />
          </div>
        {/if}

        <div class="actions">
          <button class="btn btn-ghost" on:click={close}>Annuler</button>
          <button
            class="btn btn-primary"
            on:click={doTranscribe}
            disabled={selectedIds.size === 0}
          >
            {isBatch
              ? `⚡ Lancer ${selectedIds.size} transcriptions`
              : "Lancer la transcription"}
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
    background: rgba(0, 0, 0, 0.8);
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
    width: 560px;
    max-width: calc(100vw - 32px);
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: var(--shadow-lg);
    display: flex;
    flex-direction: column;
    gap: 16px;
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

  .badge-batch {
    display: inline-block;
    margin-left: 8px;
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    background: var(--accent-subtle);
    border: 1px solid rgba(184, 255, 60, 0.3);
    border-radius: 99px;
    padding: 1px 8px;
    vertical-align: middle;
  }
  .badge-experimental {
    display: inline-block;
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    background: rgba(255, 170, 0, 0.15);
    color: var(--warning, #ffaa00);
    border: 1px solid rgba(255, 170, 0, 0.35);
    border-radius: 3px;
    padding: 1px 5px;
    margin-left: 6px;
    vertical-align: middle;
    position: relative;
    top: -1px;
  }

  .section-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .sel-btns {
    display: flex;
    gap: 5px;
  }
  .micro-btn {
    font-size: 10px;
    padding: 2px 8px;
    border: 1px solid var(--border);
    border-radius: 99px;
    background: var(--bg-elevated);
    color: var(--text-secondary);
    cursor: pointer;
  }
  .micro-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .projects-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 140px;
    overflow-y: auto;
  }
  .proj-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    cursor: pointer;
    text-align: left;
    transition: all 0.1s;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .proj-row:hover:not(.disabled) {
    border-color: var(--border-strong);
    color: var(--text-primary);
  }
  .proj-row.selected {
    border-color: rgba(184, 255, 60, 0.4);
    background: var(--accent-subtle);
    color: var(--accent);
  }
  .proj-row.disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .proj-check {
    font-size: 15px;
    flex-shrink: 0;
  }
  .proj-name {
    flex: 1;
  }
  .proj-tag {
    font-size: 9px;
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--bg-overlay);
    border: 1px solid var(--border);
    color: var(--text-muted);
  }
  .proj-tag.muted {
    opacity: 0.5;
  }

  /* [FIX BUG 4] Grille 5 colonnes pour les modèles */
  .model-grid {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 6px;
  }
  .model-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    background: var(--bg-elevated);
    border: 1.5px solid var(--border);
    border-radius: var(--radius);
    padding: 8px 4px;
    cursor: pointer;
  }
  .model-card.selected {
    border-color: var(--border-active);
    background: var(--accent-subtle);
  }
  .model-name {
    font-size: 12px;
    font-weight: 700;
  }
  .model-size {
    font-size: 9px;
    color: var(--text-muted);
  }
  .model-meta {
    font-size: 11px;
  }
  .hint {
    font-size: 11px;
    color: var(--text-muted);
    text-align: center;
  }

  .lang-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .lang-btn {
    padding: 6px 12px;
    border-radius: 99px;
    border: 1.5px solid var(--border);
    background: var(--bg-elevated);
    font-size: 12px;
    cursor: pointer;
  }
  .lang-btn.active {
    border-color: var(--border-active);
    background: var(--accent-subtle);
  }

  .toggles {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .toggle-row {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 10px 12px;
    cursor: pointer;
  }
  .toggle-name {
    font-size: 13px;
    font-weight: 600;
  }
  .toggle-desc {
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 2px;
  }

  .slider-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .slider-row label {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .slider-value {
    font-family: var(--font-mono);
    color: var(--accent);
  }
  .slider-row input[type="range"] {
    width: 100%;
    accent-color: var(--accent);
  }

  /* [FIX BUG 5] Langue détectée */
  .lang-detected {
    background: var(--accent-subtle);
    border: 1px solid rgba(184, 255, 60, 0.3);
    border-radius: var(--radius-sm);
    padding: 8px 12px;
    font-size: 12px;
    color: var(--accent);
  }

  .results {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
    text-align: center;
  }
  .result-stat {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 8px;
  }
  .stat-value {
    display: block;
    font-size: 28px;
    font-weight: 700;
    font-family: var(--font-display);
  }
  .stat-label {
    font-size: 11px;
    color: var(--text-muted);
  }

  /* [FIX BUG 3] État done sans résultat */
  .empty-done {
    background: rgba(255, 75, 75, 0.06);
    border: 1px solid rgba(255, 75, 75, 0.2);
    border-radius: var(--radius-sm);
    padding: 12px 16px;
    font-size: 13px;
    color: var(--danger, #ff4b4b);
  }

  .results-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .result-row {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 10px 14px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .result-row.result-error {
    border-color: rgba(255, 75, 75, 0.3);
  }
  .result-name {
    font-size: 13px;
    font-weight: 600;
    flex: 1;
  }
  .result-stats {
    display: flex;
    gap: 14px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .result-stats .warn {
    color: var(--warning, #f0a500);
  }
  .result-stats .danger {
    color: var(--danger, #ff4b4b);
  }
  .result-err-msg {
    font-size: 11px;
    color: var(--danger, #ff4b4b);
  }

  .batch-jobs {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .job-row {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .job-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .job-name {
    font-size: 13px;
    font-weight: 600;
  }
  .job-step {
    font-size: 11px;
    color: var(--text-muted);
  }
  .job-step.step-done {
    color: var(--success, #4bff7a);
  }
  .job-step.step-error {
    color: var(--danger, #ff4b4b);
  }
  .job-bar {
    height: 3px;
    background: var(--bg-overlay);
    border-radius: 2px;
    overflow: hidden;
  }
  .job-fill {
    height: 100%;
    border-radius: 2px;
    transition:
      width 0.4s,
      background 0.3s;
  }
  .job-fill.step-queued {
    width: 4%;
    background: var(--text-muted);
  }
  .job-fill.step-loading {
    width: 10%;
    background: var(--text-muted);
    animation: pulse 1s infinite;
  }
  .job-fill.step-extracting {
    width: 25%;
    background: var(--warning, #f0a500);
    animation: pulse 1s infinite;
  }
  .job-fill.step-transcribing {
    width: 65%;
    background: var(--accent);
    animation: pulse 1s infinite;
  }
  .job-fill.step-detecting {
    width: 88%;
    background: var(--accent);
    animation: pulse 1s infinite;
  }
  .job-fill.step-done {
    width: 100%;
    background: var(--success, #4bff7a);
  }
  .job-fill.step-error {
    width: 100%;
    background: var(--danger, #ff4b4b);
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.55;
    }
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    padding: 24px 0;
  }
  .big-spinner {
    width: 48px;
    height: 48px;
    border: 4px solid var(--border-strong);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  .step-label {
    font-size: 14px;
    font-weight: 600;
    text-align: center;
  }

  /* [FIX BUG 1] Indicateurs d'étapes visuels */
  .step-indicators {
    display: flex;
    gap: 8px;
  }
  .step-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--border-strong);
    transition: all 0.3s;
  }
  .step-dot.active {
    background: var(--accent);
    box-shadow: 0 0 6px var(--accent);
  }
  .step-dot.done {
    background: var(--success, #4bff7a);
  }

  .download-progress {
    width: 100%;
    max-width: 300px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: center;
  }
  .progress-bar {
    height: 4px;
    background: var(--bg-overlay);
    border-radius: 2px;
    overflow: hidden;
  }
  .progress-bar-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
    transition: width 0.3s;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }
</style>
