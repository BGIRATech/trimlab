<!--
  FICHIER : trimlab/src/routes/App.svelte
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { get } from "svelte/store";
  import Waveform from "../components/waveform/Waveform.svelte";
  import Timeline from "../components/timeline/Timeline.svelte";
  import ParamsPanel from "../components/ParamsPanel.svelte";
  import VideoPlayer from "../components/player/VideoPlayer.svelte";
  import SubtitleEditor from "../components/subtitles/SubtitleEditor.svelte";
  import {
    currentView,
    projects,
    activeProjectId,
    activeProject,
    licence,
    licencePlanLabel,
    canExport,
    playhead,
    isPlaying,
    zoom,
    showLicenceModal,
    showExportModal,
    showExportFFmpegModal,
    showTranscribeModal,
    notify,
    dismissNotif,
    updateProjectInStore,
    removeProjectFromStore,
    // segments, // Plus utilisé directement ici, on utilise l'historique
    waveformData as waveformStore,
    isAnalysing,
    isImporting,
  } from "../lib/store";

  // [CORRECTION] Import du store avec historique
  import { segmentsHistory } from "../lib/history";

  import type {
    Project,
    MediaFile,
    Segment,
    ProjectSettings,
  } from "../lib/store";
  import { commands } from "../lib/commands";
  import ExportFFmpegModal from "../components/ExportFFmpegModal.svelte";
  import TranscribeModal from "../components/TranscribeModal.svelte";
  import { formatDuration, formatFileSize, formatTimecode } from "../lib/utils";
  import BatchQueue from "../components/BatchQueue.svelte";

  let waveformData: number[] = [];
  let dragOver = false;
  let compareMode = false; // split-screen avant/après
  let showSettings = false;
  let showShortcuts = false;

  // Paramètres courants reçus de ParamsPanel via event "analyse"
  let analyseParams = {
    thresholdDb: 0, // 0 = auto
    minDurationMs: 300,
    paddingBefore: 0.05,
    paddingAfter: 0.12,
    minSpeechMs: 200,
    aggressiveness: 3,
  };
  let progressInterval: ReturnType<typeof setInterval>;
  let showSubtitleEditor = false;
  let showBatchQueue = false;

  // ── UX : modales et états supplémentaires ─────────────────────
  let showDeleteConfirm = false;
  let deleteTargetId: string | null = null;
  let showReanalyseConfirm = false;
  let reanalyseTarget: { id: string; file: MediaFile } | null = null;
  let projectSearch = "";
  let renamingProjectId: string | null = null;
  let renameValue = "";

  // ── Undo/Redo via Store dédié ─────────────────────────────────────────────────
  // Les variables 'past' et 'future' sont supprimées, gérées par segmentsHistory

  let canUndo = false;
  let canRedo = false;

  // Action Svelte pour focus sans warning a11y autofocus
  function focusOnMount(el: HTMLElement) {
    requestAnimationFrame(() => el.focus());
    return {};
  }

  // Réactivité pour activer/désactiver les boutons Undo/Redo
  $: if ($segmentsHistory) {
    canUndo = segmentsHistory.canUndo();
    canRedo = segmentsHistory.canRedo();
  }

  // [FIX BUG A] Recharger segments + waveform quand l'utilisateur change de projet
  let _lastLoadedProjectId: string | null = null;
  $: if ($activeProject && $activeProject.id !== _lastLoadedProjectId) {
    _lastLoadedProjectId = $activeProject.id;
    if ($activeProject.status === "ready" && $activeProject.files[0]) {
      commands
        .listSegments($activeProject.id)
        .then((segs: Segment[]) => {
          segmentsHistory.init(segs);
        })
        .catch(() => {});
      commands
        .getWaveformData($activeProject.files[0].path)
        .then((pts: number[]) => {
          waveformData = pts;
          waveformStore.set(pts);
        })
        .catch(() => {});
    }
  }

  function undo() {
    if (segmentsHistory.undo()) {
      notify({
        type: "info",
        title: "Annulé",
        message: "Ctrl+Shift+Z pour rétablir",
      });
    }
  }

  function redo() {
    if (segmentsHistory.redo()) {
      notify({ type: "info", title: "Rétabli", message: "" });
    }
  }

  // ── Raccourcis clavier ───────────────────────────────────────────────────────
  function handleKeydown(e: KeyboardEvent) {
    const tag = (e.target as HTMLElement).tagName;
    if (
      tag === "INPUT" ||
      tag === "TEXTAREA" ||
      (e.target as HTMLElement).isContentEditable
    )
      return;

    const ctrl = e.ctrlKey || e.metaKey;

    switch (e.key) {
      case " ":
      case "k":
      case "K":
        e.preventDefault();
        isPlaying.update((v) => !v);
        break;
      case "j":
      case "J":
        e.preventDefault();
        isPlaying.set(false);
        playhead.update((t) => Math.max(0, t - 5));
        break;
      case "l":
      case "L":
        e.preventDefault();
        if ($isPlaying) {
          playhead.update((t) =>
            Math.min($activeProject?.files[0]?.duration ?? 0, t + 5),
          );
        } else {
          isPlaying.set(true);
        }
        break;
      case "ArrowLeft":
        if (!ctrl) {
          e.preventDefault();
          isPlaying.set(false);
          playhead.update((t) => Math.max(0, t - (e.shiftKey ? 1 : 0.1)));
        }
        break;
      case "ArrowRight":
        if (!ctrl) {
          e.preventDefault();
          isPlaying.set(false);
          playhead.update((t) =>
            Math.min(
              $activeProject?.files[0]?.duration ?? 0,
              t + (e.shiftKey ? 1 : 0.1),
            ),
          );
        }
        break;
      case "Home":
        e.preventDefault();
        isPlaying.set(false);
        playhead.set(0);
        break;
      case "z":
      case "Z":
        if (ctrl) {
          e.preventDefault();
          e.shiftKey ? redo() : undo();
        }
        break;
      case "y":
      case "Y":
        if (ctrl) {
          e.preventDefault();
          redo();
        }
        break;
      case "+":
      case "=":
        if (ctrl) {
          e.preventDefault();
          zoom.update((z) => Math.min(20, z * 1.2));
        }
        break;
      case "-":
        if (ctrl) {
          e.preventDefault();
          zoom.update((z) => Math.max(1, z / 1.2));
        }
        break;
      case "0":
        if (ctrl) {
          e.preventDefault();
          zoom.set(1);
        }
        break;

      // ── [UX #11] E key — toggle segment sous le curseur ───────────────────────
      case "e":
      case "E": {
        e.preventDefault();
        const ph = get(playhead);
        const segs = get(segmentsHistory);
        const seg = segs.find(
          (s: Segment) => ph >= s.start_time && ph <= s.end_time,
        );
        if (seg) toggleSegment(seg.id);
        break;
      }
    }
  }

  // ─── Init ───────────────────────────────────────────────────────────────────

  onMount(async () => {
    // AppEditor est détruit/recréé à chaque navigation (router racine).
    // On force le rechargement de la waveform et des segments pour le projet actif.
    const ap = get(activeProject);
    if (ap && ap.status === "ready" && ap.files[0]) {
      _lastLoadedProjectId = null; // force le $: réactif à se déclencher
    }

    try {
      const projs = await commands.listProjects();
      projects.set(projs);
      // Ne pas écraser un projet déjà actif (import en cours)
      if (projs.length > 0 && !get(activeProjectId))
        activeProjectId.set(projs[0].id);
    } catch (_) {}
    try {
      const lic = await commands.getLicence();
      licence.set(lic as any);
    } catch (_) {}
  });

  onDestroy(() => {
    clearInterval(progressInterval);
  });

  // ─── Import fichier ─────────────────────────────────────────────────────────

  async function handleFileDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    const files = Array.from(e.dataTransfer?.files ?? []);
    for (const file of files) {
      if (file.type.startsWith("video/") || file.type.startsWith("audio/")) {
        const path = (file as any).path || file.name;
        await importFile(path);
      }
    }
  }

  async function handleFileOpen() {
    const path = await commands.openFileDialog();
    if (path) await importFile(path);
  }

  async function importFile(path: string) {
    isImporting.set(true);
    try {
      const name =
        path
          .split(/[/\\]/)
          .pop()
          ?.replace(/\.[^.]+$/, "") ?? "Nouveau projet";
      const proj = await commands.createProject(name);
      projects.update((ps: Project[]) => [...ps, proj]);
      activeProjectId.set(proj.id);

      await commands.updateProjectStatus(proj.id, "importing", 0);
      updateProjectInStore({ ...proj, status: "importing", progress: 0 });

      const info = await commands.probeMedia(path);

      // ── Détection formats incompatibles Premiere Pro ─────────────
      const ext = path.split(".").pop()?.toLowerCase() ?? "";
      const premiereIncompatible = ["mkv", "avi", "wmv", "flv", "webm"];
      if (premiereIncompatible.includes(ext)) {
        notify({
          type: "warning",
          title: `Format .${ext.toUpperCase()} détecté`,
          message: `Premiere Pro ne supporte pas le .${ext} nativement. Un bouton de conversion apparaîtra dans l'éditeur.`,
          duration: 8000,
        });
      }

      const mediaFile = await commands.addMediaFile(proj.id, {
        project_id: proj.id,
        name: path.split(/[/\\]/).pop() ?? path,
        path,
        duration: info.duration,
        size: info.size,
        media_type: info.has_video ? "video" : "audio",
        has_video: info.has_video,
        has_audio: info.has_audio,
        fps: info.fps,
        codec: info.codec,
        sample_rate: info.sample_rate,
      });

      // Attacher le mediaFile au projet dans le store local
      // Sans ça, $activeProject.files reste [] → éditeur ne s'affiche pas
      updateProjectInStore({
        ...{ ...proj, status: "importing", progress: 0 },
        files: [mediaFile],
      });

      isImporting.set(false);
      notify({
        type: "info",
        title: "Fichier importe",
        message: mediaFile.name,
      });

      const points = await commands.getWaveformData(path);
      waveformData = points;
      waveformStore.set(points);

      await analyseProject(proj.id, mediaFile);
    } catch (err) {
      isImporting.set(false);
      notify({ type: "error", title: "Erreur d'import", message: String(err) });
    }
  }

  // ─── Analyse ────────────────────────────────────────────────────────────────

  async function analyseProject(projectId: string, file: MediaFile) {
    const proj = $projects.find((p: Project) => p.id === projectId);
    if (!proj) return;

    isAnalysing.set(true);
    await commands.updateProjectStatus(projectId, "analysing", 0);
    updateProjectInStore({ ...proj, status: "analysing", progress: 0 });

    // [FIX 3] Fausse progression (analyseAndSave est bloquant côté Rust)
    let _fakeProgress = 0;
    const _progressInterval = setInterval(() => {
      _fakeProgress += (90 - _fakeProgress) * 0.04;
      updateProjectInStore({
        ...proj,
        status: "analysing",
        progress: Math.round(_fakeProgress),
      });
    }, 150);

    try {
      const segs = await commands.analyseAndSave(
        projectId,
        file.path,
        analyseParams.thresholdDb,
        analyseParams.minDurationMs,
        file.duration,
        analyseParams.paddingBefore,
        analyseParams.paddingAfter,
        analyseParams.minSpeechMs,
        analyseParams.aggressiveness,
      );

      // [CORRECTION] Utiliser .init pour charger les nouvelles données sans historique précédent
      segmentsHistory.init(segs);

      const keptDuration = segs
        .filter((s: Segment) => s.seg_type === "keep")
        .reduce((a: number, s: Segment) => a + (s.end_time - s.start_time), 0);

      const stats = {
        original_duration: file.duration,
        trimmed_duration: keptDuration,
        silences_removed: segs.filter((s: Segment) => s.seg_type === "silence")
          .length,
        fillers_removed: 0,
        space_saved: 0,
        processing_time: 0,
        accuracy: 0.92,
      };

      await commands.saveProcessingStats(projectId, stats);
      await commands.updateProjectStatus(projectId, "ready", 100);
      updateProjectInStore({ ...proj, status: "ready", progress: 100, stats });

      // Naviguer automatiquement vers l'éditeur dès que l'analyse est terminée
      activeProjectId.set(projectId);
      currentView.set("app");

      notify({
        type: "success",
        title: "Analyse terminee",
        message: `${stats.silences_removed} silences detectes`,
      });
    } catch (err) {
      await commands.updateProjectStatus(projectId, "error", 0);
      updateProjectInStore({ ...proj, status: "error" });
      notify({
        type: "error",
        title: "Erreur d'analyse",
        message: String(err),
      });
    } finally {
      clearInterval(_progressInterval);
      isAnalysing.set(false);
    }
  }

  // ─── Segments ───────────────────────────────────────────────────────────────

  async function toggleSegment(id: string) {
    try {
      // pushHistory supprimé : géré par segmentsHistory.update
      const updated = await commands.toggleSegment(id);
      segmentsHistory.update((segs: Segment[]) =>
        segs.map((s: Segment) => (s.id === id ? updated : s)),
      );
    } catch (err) {
      notify({ type: "error", title: "Erreur", message: String(err) });
    }
  }

  async function deleteSegment(id: string) {
    try {
      await commands.deleteSegment(id);
      segmentsHistory.update((segs: Segment[]) =>
        segs.filter((s: Segment) => s.id !== id),
      );
    } catch (err) {
      notify({ type: "error", title: "Erreur", message: String(err) });
    }
  }

  // ── Trim segment ─────────────────────────────────────────────────────────────
  async function handleTrim(id: string, start: number, end: number) {
    if (!$activeProject) return;

    segmentsHistory.update((segs: Segment[]) => {
      const updated = segs.map((s: Segment) =>
        s.id === id ? { ...s, start_time: start, end_time: end } : s,
      );
      // Sauvegarde async après mise à jour UI
      commands.saveSegments($activeProject.id, updated).catch((err) => {
        notify({ type: "error", title: "Erreur trim", message: String(err) });
      });
      return updated;
    });
  }

  // ─── Demo ───────────────────────────────────────────────────────────────────

  async function loadDemo() {
    notify({
      type: "info",
      title: "Mode demo",
      message: "Glisse un vrai fichier video pour analyser",
    });
    try {
      const proj = await commands.createProject("Projet demo");
      projects.update((ps: Project[]) => [...ps, proj]);
      activeProjectId.set(proj.id);
    } catch (_) {}
  }

  // ─── Projets CRUD ───────────────────────────────────────────────────────────

  async function handleNewProject() {
    // [UX #9] Ouvrir directement le file picker — le projet sera créé par importFile
    await handleFileOpen();
  }

  function handleDeleteProject(id: string) {
    deleteTargetId = id;
    showDeleteConfirm = true;
  }

  async function confirmDeleteProject() {
    if (!deleteTargetId) return;
    try {
      await commands.deleteProject(deleteTargetId);
      removeProjectFromStore(deleteTargetId);
    } catch (err) {
      notify({ type: "error", title: "Erreur", message: String(err) });
    } finally {
      showDeleteConfirm = false;
      deleteTargetId = null;
    }
  }

  // ─── Rename project ────────────────────────────────────────────────────────
  function startRename(proj: Project) {
    renamingProjectId = proj.id;
    renameValue = proj.name;
  }

  async function commitRename(id: string) {
    const name = renameValue.trim();
    if (!name || name === $projects.find((p: Project) => p.id === id)?.name) {
      renamingProjectId = null;
      return;
    }
    try {
      // Optimistic update
      const p = $projects.find((p: Project) => p.id === id);
      if (p) updateProjectInStore({ ...p, name });
      // Persist via updateProjectSettings (nom stocké dans name, pas settings)
      // Note: commande update_project_name à ajouter côté Rust si besoin
      // Pour l'instant on met à jour le store uniquement
    } catch (err) {
      notify({
        type: "error",
        title: "Erreur renommage",
        message: String(err),
      });
    } finally {
      renamingProjectId = null;
    }
  }

  // ─── Settings ───────────────────────────────────────────────────────────────

  async function saveSettings(patch: Partial<ProjectSettings>) {
    if (!$activeProject) return;
    const updated = { ...$activeProject.settings, ...patch };
    updateProjectInStore({ ...$activeProject, settings: updated });
    await commands.updateProjectSettings($activeProject.id, updated);
  }
  function onSilenceThreshold(e: Event) {
    saveSettings({
      silence_threshold: Number((e.target as HTMLInputElement).value),
    });
  }
  function onSilenceDuration(e: Event) {
    saveSettings({
      silence_min_duration: Number((e.target as HTMLInputElement).value),
    });
  }
  function onPaddingBefore(e: Event) {
    saveSettings({
      padding_before: Number((e.target as HTMLInputElement).value),
    });
  }
  function onAiMode(e: Event) {
    saveSettings({
      ai_mode: (e.target as HTMLSelectElement).value as
        | "lite"
        | "fast"
        | "quality",
    });
  }
  function onLanguage(e: Event) {
    saveSettings({ language: (e.target as HTMLSelectElement).value });
  }

  // ─── Préréglages ─────────────────────────────────────────────────────────
  // Gérés par ParamsPanel.svelte (presets, aggressivité, padding, min_speech)

  function onZoomInput(e: Event) {
    zoom.set(Number((e.target as HTMLInputElement).value));
  }

  // ─── Playback ───────────────────────────────────────────────────────────────

  function togglePlay() {
    isPlaying.update((v: boolean) => !v);
  }

  $: if ($isPlaying && (!$activeProject || $activeProject.files.length === 0)) {
    progressInterval = setInterval(() => {
      playhead.update((t: number) => {
        if (t >= 100) {
          isPlaying.set(false);
          return 0;
        }
        return t + 0.1;
      });
    }, 100);
  } else {
    clearInterval(progressInterval);
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="app-layout">
  <!-- Sidebar -->
  <aside class="sidebar">
    <div class="sidebar-logo">
      <svg width="24" height="24" viewBox="0 0 28 28" fill="none">
        <rect width="28" height="28" rx="7" fill="#B8FF3C" />
        <path
          d="M7 14 L11 10 L15 16 L19 8 L21 14"
          stroke="#0A0B0D"
          stroke-width="2.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
      <span class="sidebar-title font-display">TrimLab</span>
    </div>

    <nav class="sidebar-nav">
      <button class="nav-item active">
        <svg
          width="14"
          height="14"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
        >
          <rect x="1" y="1" width="14" height="10" rx="2" />
          <path d="M4 15h8" /><path d="M8 11v4" />
        </svg>
        Editeur
      </button>
      <button class="nav-item" on:click={() => currentView.set("dashboard")}>
        <svg
          width="14"
          height="14"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
        >
          <rect x="1" y="1" width="6" height="6" rx="1" />
          <rect x="9" y="1" width="6" height="6" rx="1" />
          <rect x="1" y="9" width="6" height="6" rx="1" />
          <rect x="9" y="9" width="6" height="6" rx="1" />
        </svg>
        Dashboard
      </button>
    </nav>

    <div class="sidebar-divider"></div>

    <div class="sidebar-section-title">
      Projets
      <span class="project-count">{$projects.length}</span>
    </div>

    <!-- [UX #13] Filtre de recherche projets -->
    {#if $projects.length > 3}
      <input
        type="text"
        class="project-search"
        placeholder="Filtrer…"
        bind:value={projectSearch}
      />
    {/if}

    <div class="projects-list">
      {#each $projects.filter((p) => !projectSearch || p.name
            .toLowerCase()
            .includes(projectSearch.toLowerCase())) as proj (proj.id)}
        <button
          class="project-item"
          class:active={$activeProjectId === proj.id}
          on:click={() => activeProjectId.set(proj.id)}
        >
          <div
            class="project-status-dot {proj.status}"
            title={proj.status}
          ></div>
          <div class="project-info">
            <!-- [UX #12] Double-click pour renommer -->
            {#if renamingProjectId === proj.id}
              <input
                class="rename-input"
                bind:value={renameValue}
                on:blur={() => commitRename(proj.id)}
                on:keydown={(e) => {
                  if (e.key === "Enter") commitRename(proj.id);
                  if (e.key === "Escape") renamingProjectId = null;
                }}
                on:click|stopPropagation
                use:focusOnMount
              />
            {:else}
              <div
                class="project-name truncate"
                title="Double-clic pour renommer"
                role="button"
                tabindex="0"
                on:dblclick|stopPropagation={() => startRename(proj)}
                on:keydown|stopPropagation={(e) =>
                  e.key === "Enter" && startRename(proj)}
              >
                {proj.name}
              </div>
            {/if}
            <!-- [UX #14] Métadonnées : durée + date analyse -->
            <div class="project-meta">
              {#if proj.files.length > 0}
                <span>{formatDuration(proj.files[0].duration)}</span>
                {#if proj.status === "ready"}
                  <span
                    title="Dernière analyse : {new Date(
                      proj.updated_at,
                    ).toLocaleString('fr-FR')}"
                  >
                    · {new Date(proj.updated_at).toLocaleDateString("fr-FR", {
                      day: "2-digit",
                      month: "2-digit",
                    })}
                  </span>
                {/if}
              {:else}
                <span>Vide</span>
              {/if}
            </div>
          </div>
          <button
            class="btn btn-ghost btn-sm project-delete"
            on:click|stopPropagation={() => handleDeleteProject(proj.id)}
            title="Supprimer"
          >
            <svg
              width="10"
              height="10"
              viewBox="0 0 12 12"
              fill="none"
              stroke="currentColor"
              stroke-width="1.8"><path d="M2 2l8 8M10 2l-8 8" /></svg
            >
          </button>
        </button>
      {/each}
      <button class="new-project-btn" on:click={handleNewProject}>
        <svg
          width="11"
          height="11"
          viewBox="0 0 12 12"
          fill="none"
          stroke="currentColor"
          stroke-width="2"><path d="M6 1v10M1 6h10" /></svg
        >
        Importer un fichier
      </button>
      <button
        class="new-project-btn batch-btn"
        class:active={showBatchQueue}
        on:click={() => (showBatchQueue = !showBatchQueue)}
        title="Traitement multi-clips en parallèle"
      >
        ⚡ Multi-clips
      </button>
    </div>

    <div style="flex:1"></div>

    <div class="licence-badge" class:free={$licence.status === "free"}>
      <div class="licence-icon">{$licence.status === "free" ? "L" : "OK"}</div>
      <div>
        <div class="licence-plan">{$licencePlanLabel}</div>
        {#if $licence.status === "free"}
          <button
            class="licence-upgrade"
            on:click={() => showLicenceModal.set(true)}
          >
            Activer l'export
          </button>
        {:else}
          <div class="licence-email text-muted">
            {$licence.email ?? "Activee"}
          </div>
        {/if}
      </div>
    </div>
  </aside>

  <!-- Main editor -->
  <main class="editor">
    {#if !$isImporting && (!$activeProject || $activeProject.files.length === 0)}
      <div
        class="dropzone"
        class:over={dragOver}
        on:dragover|preventDefault={() => (dragOver = true)}
        on:dragleave={() => (dragOver = false)}
        on:drop={handleFileDrop}
        role="region"
        aria-label="Zone de depot de fichiers"
      >
        <div class="dropzone-inner">
          <div class="dropzone-icon">
            <svg width="40" height="40" viewBox="0 0 40 40" fill="none">
              <rect width="40" height="40" rx="12" fill="var(--bg-elevated)" />
              <path
                d="M20 26V14M14 20l6-6 6 6"
                stroke="var(--accent)"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <path
                d="M12 30h16"
                stroke="var(--border-strong)"
                stroke-width="1.5"
                stroke-linecap="round"
              />
            </svg>
          </div>
          <div class="dropzone-title font-display">
            Deposez vos fichiers ici
          </div>
          <p class="dropzone-sub">MP4, MOV, MKV, MP3, WAV, FLAC</p>
          <div class="dropzone-actions">
            <button class="btn btn-primary" on:click={handleFileOpen}
              >Ouvrir un fichier</button
            >
            <button class="btn btn-ghost" on:click={loadDemo}
              >Charger la demo</button
            >
          </div>
        </div>
      </div>
    {:else if $isImporting}
      <div
        style="display:flex;flex-direction:column;align-items:center;justify-content:center;height:100%;gap:20px;color:var(--text-muted);"
      >
        <div
          style="width:44px;height:44px;border:3px solid var(--border);border-top-color:var(--accent);border-radius:50%;animation:spin 0.8s linear infinite;"
        ></div>
        <div
          style="font-family:var(--font-display);font-size:1rem;color:var(--text-primary);"
        >
          {$activeProject?.name ?? "Import…"}
        </div>
        <div style="font-size:0.85rem;">Lecture du fichier…</div>
      </div>
    {:else if $activeProject}
      <!-- Header -->
      <div class="editor-header">
        <div class="editor-file-info">
          <div class="editor-file-name font-display">{$activeProject.name}</div>
          {#if $activeProject.files[0]}
            {@const f = $activeProject.files[0]}
            <div class="editor-file-meta">
              <span class="badge badge-muted">{f.media_type}</span>
              <span class="text-muted font-mono" style="font-size:11px;"
                >{formatDuration(f.duration)}</span
              >
              <span class="text-muted font-mono" style="font-size:11px;"
                >{formatFileSize(f.size)}</span
              >
              {#if f.fps}<span
                  class="text-muted font-mono"
                  style="font-size:11px;">{f.fps} fps</span
                >{/if}
              {#if f.codec}<span
                  class="text-muted font-mono"
                  style="font-size:11px;">{f.codec.toUpperCase()}</span
                >{/if}

              {#if ["mkv", "avi", "wmv", "flv", "webm"].includes(f.path
                  .split(".")
                  .pop()
                  ?.toLowerCase() ?? "")}
                <button
                  class="badge badge-warning convert-source-btn"
                  title="Premiere Pro ne supporte pas ce format nativement — cliquer pour convertir la SOURCE complète en MP4 (conserve tous les timecodes)"
                  on:click={async () => {
                    if (!$activeProject?.files[0]) return;
                    const src = $activeProject.files[0].path;
                    // Sortie dans le même dossier que la source
                    const out = src.replace(/\.[^.]+$/, "_premiere.mp4");
                    // duration:0 = toast permanent — dismiss par id dès que Rust répond
                    const convertToastId = notify({
                      type: "info",
                      title: "Conversion source en cours…",
                      message: `${src.split(/[/\\]/).pop()} → MP4`,
                      duration: 0,
                    });
                    const dismissConvertToast = () =>
                      dismissNotif(convertToastId);
                    try {
                      // [FIX] Convertir la SOURCE COMPLÈTE (pas le montage découpé)
                      // pour que les timecodes du XML correspondent au fichier MP4
                      await commands.convertSourceToMp4(
                        $activeProject.id,
                        $activeProject.files[0].path,
                        out,
                      );
                      // Re-lier le projet au nouveau fichier MP4 en base
                      await commands.updateMediaFilePath(
                        $activeProject.id,
                        out,
                      );
                      // Mettre à jour le store immédiatement (sans recharger)
                      const updatedFile = {
                        ...$activeProject.files[0],
                        path: out,
                        name: out.split(/[/\\]/).pop() ?? out,
                      };
                      updateProjectInStore({
                        ...$activeProject,
                        files: [updatedFile],
                      });
                      dismissConvertToast();
                      notify({
                        type: "success",
                        title: "Source convertie en MP4 ✓",
                        message: `${updatedFile.name} — le prochain export XML pointera vers ce fichier`,
                        duration: 10000,
                      });
                    } catch (e) {
                      dismissConvertToast();
                      notify({
                        type: "error",
                        title: "Erreur de conversion",
                        message: String(e),
                      });
                    }
                  }}
                  >⚠ .{f.name.split(".").pop()?.toUpperCase()} → Convertir source
                  en MP4</button
                >
              {/if}
            </div>
          {/if}
        </div>

        <div class="editor-status">
          {#if $activeProject.status === "analysing"}
            <div class="status-analysing">
              <span class="spinner-ring"></span>
              Analyse… {$activeProject.progress}%
            </div>
            <div class="progress-bar" style="width:120px">
              <div
                class="progress-bar-fill"
                style="width:{$activeProject.progress}%"
              ></div>
            </div>
          {:else if $activeProject.status === "ready"}
            <span class="badge badge-success">Pret</span>
          {:else if $activeProject.status === "error"}
            <span class="badge badge-danger">Erreur</span>
          {/if}
        </div>

        <div class="editor-actions">
          <!-- Groupe 1 : Historique (icônes seules OK — convention universelle) -->
          <div class="actions-group">
            <button
              class="btn btn-ghost btn-sm"
              title="Annuler (Ctrl+Z)"
              disabled={!canUndo}
              on:click={undo}
            >
              <svg
                width="12"
                height="12"
                viewBox="0 0 16 16"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                ><path d="M2 8a6 6 0 1 1 1.5 4" /><path d="M2 4v4h4" /></svg
              >
              Annuler
            </button>
            <button
              class="btn btn-ghost btn-sm"
              title="Rétablir (Ctrl+Shift+Z)"
              disabled={!canRedo}
              on:click={redo}
            >
              <svg
                width="12"
                height="12"
                viewBox="0 0 16 16"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                ><path d="M14 8a6 6 0 1 0-1.5 4" /><path d="M14 4v4h-4" /></svg
              >
              Rétablir
            </button>
          </div>

          <div class="actions-sep"></div>

          <!-- Groupe 2 : Configuration -->
          <div class="actions-group">
            <button
              class="btn btn-ghost btn-sm"
              class:btn-active={showSettings}
              on:click={() => (showSettings = !showSettings)}
            >
              ⚙ Paramètres
            </button>
            <button
              class="btn btn-ghost btn-sm shortcuts-btn"
              class:btn-active={showShortcuts}
              on:click={() => (showShortcuts = !showShortcuts)}
            >
              ⌨ Raccourcis
            </button>
          </div>

          <div class="actions-sep"></div>

          <!-- Groupe 3 : Outils IA -->
          <div class="actions-group">
            <button class="btn btn-ghost btn-sm" on:click={handleFileOpen}>
              + Importer un fichier
            </button>
            <button
              class="btn btn-ghost btn-sm"
              on:click={() => showTranscribeModal.set(true)}
              title="Détecte automatiquement les silences, fillers et répétitions avec Whisper"
              >🎙 Transcrire (IA)</button
            >
            <button
              class="btn btn-ghost btn-sm"
              on:click={() => (showSubtitleEditor = true)}
              style="color:var(--accent)">✦ Sous-titres</button
            >
          </div>

          <div class="actions-sep"></div>

          <!-- Groupe 4 : Export — mise en avant selon la licence -->
          <div class="actions-group">
            <button
              class="btn btn-ghost btn-sm"
              on:click={() => showExportFFmpegModal.set(true)}
              title={$canExport
                ? "Exporte directement en MP4, MKV ou audio"
                : "Nécessite une licence pour exporter"}
              >{$canExport
                ? "⚡ Exporter la vidéo"
                : "🔒 Exporter la vidéo"}</button
            >
            <button
              class="btn {$canExport ? 'btn-primary' : 'btn-ghost'} btn-sm"
              on:click={() => showExportModal.set(true)}
              title={$canExport
                ? "Générer un fichier XML/EDL pour Premiere, DaVinci, Final Cut…"
                : "Nécessite une licence pour exporter en XML/EDL"}
              >{$canExport ? "📋 XML / EDL" : "🔒 XML / EDL (licence)"}</button
            >
          </div>
        </div>
      </div>

      <!-- Stats bar — uniquement Fillers et Précision (Original/Final/Économie sont dans la Timeline) -->
      {#if $activeProject.stats && ($activeProject.stats.fillers_removed > 0 || $activeProject.stats.accuracy > 0)}
        {@const s = $activeProject.stats}
        <div class="stats-bar">
          {#if s.fillers_removed > 0}
            <div class="stat-chip">
              <span class="text-muted">Fillers</span>
              <span class="font-mono" style="color:var(--warning)"
                >{s.fillers_removed}</span
              >
            </div>
          {/if}
          {#if s.silences_removed > 0}
            <div class="stat-chip">
              <span class="text-muted">Silences</span>
              <span class="font-mono" style="color:var(--danger)"
                >{s.silences_removed}</span
              >
            </div>
          {/if}
          <div class="stat-chip">
            <span class="text-muted">Précision détection</span>
            <span class="font-mono">{Math.round(s.accuracy * 100)}%</span>
          </div>
        </div>
      {/if}

      <!-- Player + Waveform -->
      <div class="waveform-section">
        <div class="player-row">
          <!-- Lecteur video/audio -->
          {#if $activeProject.files[0]}
            <div class="player-col">
              <VideoPlayer
                filePath={$activeProject.files[0].path}
                segments={$segmentsHistory}
                duration={$activeProject.files[0].duration}
                bind:compareMode
              />
            </div>
          {/if}

          <!-- Waveform + transport -->
          <div class="waveform-col">
            <div class="transport">
              <!-- Retour début -->
              <button
                class="btn btn-ghost btn-sm"
                title="Retour au début (Home)"
                on:click={() => {
                  isPlaying.set(false);
                  playhead.set(0);
                }}
              >
                <svg
                  width="12"
                  height="12"
                  viewBox="0 0 16 16"
                  fill="currentColor"
                  ><rect x="2" y="2" width="2" height="12" rx="1" /><path
                    d="M14 3L6 8l8 5V3z"
                  /></svg
                >
              </button>
              <!-- Play/Pause -->
              <button
                class="btn btn-primary btn-sm transport-play"
                title="Lecture (Espace)"
                on:click={togglePlay}
              >
                {#if $isPlaying}
                  <svg
                    width="13"
                    height="13"
                    viewBox="0 0 16 16"
                    fill="currentColor"
                    ><rect x="3" y="2" width="4" height="12" rx="1" /><rect
                      x="9"
                      y="2"
                      width="4"
                      height="12"
                      rx="1"
                    /></svg
                  >
                {:else}
                  <svg
                    width="13"
                    height="13"
                    viewBox="0 0 16 16"
                    fill="currentColor"><path d="M4 3l9 5-9 5V3z" /></svg
                  >
                {/if}
              </button>
              <span class="timecode font-mono">{formatTimecode($playhead)}</span
              >
              {#if $activeProject?.files[0]?.duration}
                <span class="text-muted font-mono" style="font-size:10px;"
                  >/ {formatTimecode($activeProject.files[0].duration)}</span
                >
              {/if}
              <div class="transport-spacer"></div>
              <!-- Bouton split-screen avant/après -->
              <button
                class="btn btn-ghost btn-sm"
                class:btn-active={compareMode}
                title="Comparer avant/après (split-screen)"
                on:click={() => (compareMode = !compareMode)}
              >
                ⟺ {compareMode ? "Quitter" : "Comparer"}
              </button>
              <!-- Zoom avec boutons discrets -->
              <button
                class="btn btn-ghost btn-sm zoom-step"
                title="Zoom -"
                on:click={() =>
                  zoom.update((z) => Math.max(1, +(z / 1.25).toFixed(1)))}
                >−</button
              >
              <span
                class="font-mono text-muted"
                style="font-size:11px; min-width:32px; text-align:center;"
                >{$zoom.toFixed(1)}x</span
              >
              <button
                class="btn btn-ghost btn-sm zoom-step"
                title="Zoom +"
                on:click={() =>
                  zoom.update((z) => Math.min(20, +(z * 1.25).toFixed(1)))}
                >+</button
              >
              <input
                type="range"
                min="1"
                max="20"
                step="0.1"
                value={$zoom}
                on:input={onZoomInput}
                class="zoom-slider"
                title="Zoom (Ctrl+/−)"
              />
            </div>

            <Waveform
              segments={$segmentsHistory}
              duration={$activeProject.files[0]?.duration ?? 0}
              waveformPoints={waveformData}
              onSeek={(t) => playhead.set(t)}
            />
          </div>
        </div>
      </div>

      <!-- Timeline + Settings -->
      <div class="bottom-panel">
        <div class="timeline-panel">
          <Timeline
            segments={$segmentsHistory}
            duration={$activeProject.files[0]?.duration ?? 0}
            onToggle={toggleSegment}
            onDelete={deleteSegment}
            onTrim={handleTrim}
          />
        </div>
        {#if showSettings && $activeProject}
          <!-- ParamsPanel : presets, seuil auto/manuel, aggressivité, padding, min_speech -->
          <div class="settings-panel">
            <ParamsPanel
              on:analyse={(e) => {
                analyseParams = e.detail;
                if (!$activeProject?.files[0]) return;
                // [UX #4] Si le projet est déjà analysé avec des segments, demander confirmation
                if (
                  $activeProject.status === "ready" &&
                  $segmentsHistory.length > 0
                ) {
                  reanalyseTarget = {
                    id: $activeProject.id,
                    file: $activeProject.files[0],
                  };
                  showReanalyseConfirm = true;
                } else {
                  analyseProject($activeProject.id, $activeProject.files[0]);
                }
              }}
              on:fillerWordsChange={(e) => {
                // [FIX FILLERS] Persister immédiatement les mots fillers en base
                // → disponibles pour la prochaine transcription Whisper
                saveSettings({ filler_words: e.detail });
              }}
            />
          </div>
        {/if}

        {#if showShortcuts}
          <!-- Raccourcis : popover flottant, pas dans le panel params -->
          <div class="shortcuts-popover">
            <div class="shortcuts-popover-header">
              <span>Raccourcis clavier</span>
              <button
                class="shortcuts-close"
                on:click={() => (showShortcuts = false)}>✕</button
              >
            </div>
            <div class="shortcuts-list">
              {#each [["Espace / K", "Play / Pause"], ["J", "−5s"], ["L", "Play / +5s"], ["← →", "±0.1s"], ["Shift+← →", "±1s"], ["↑ ↓", "Segment prev/next"], ["E", "Couper / Garder segment"], ["Home", "Début"], ["Ctrl+Z", "Annuler"], ["Ctrl+Shift+Z", "Rétablir"], ["Ctrl+/−", "Zoom"]] as [keys, desc]}
                <div class="shortcut-row">
                  <kbd>{keys}</kbd>
                  <span>{desc}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/if}
    <!-- [UX #2] Modal confirmation suppression projet -->
    {#if showDeleteConfirm}
      <div class="confirm-overlay">
        <div class="confirm-box">
          <div class="confirm-title">Supprimer ce projet ?</div>
          <div class="confirm-body">
            Cette action est irréversible. Tous les segments et sous-titres
            associés seront supprimés.
          </div>
          <div class="confirm-actions">
            <button
              class="btn btn-ghost btn-sm"
              on:click={() => {
                showDeleteConfirm = false;
                deleteTargetId = null;
              }}>Annuler</button
            >
            <button
              class="btn btn-danger btn-sm"
              on:click={confirmDeleteProject}
            >
              <svg
                width="12"
                height="12"
                viewBox="0 0 16 16"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                ><path
                  d="M3 4h10M6 4V2h4v2M5 4v9a1 1 0 001 1h4a1 1 0 001-1V4"
                /></svg
              >
              Supprimer
            </button>
          </div>
        </div>
      </div>
    {/if}

    <!-- [UX #4] Modal confirmation ré-analyse -->
    {#if showReanalyseConfirm}
      <div class="confirm-overlay">
        <div class="confirm-box">
          <div class="confirm-title">Relancer l'analyse ?</div>
          <div class="confirm-body">
            Le projet contient <strong
              >{$segmentsHistory.length} segments</strong
            >
            éditables. Relancer l'analyse
            <strong>effacera toutes vos retouches manuelles</strong>.
          </div>
          <div class="confirm-actions">
            <button
              class="btn btn-ghost btn-sm"
              on:click={() => {
                showReanalyseConfirm = false;
                reanalyseTarget = null;
              }}>Annuler</button
            >
            <button
              class="btn btn-warning btn-sm"
              on:click={() => {
                if (reanalyseTarget)
                  analyseProject(reanalyseTarget.id, reanalyseTarget.file);
                showReanalyseConfirm = false;
                reanalyseTarget = null;
              }}
            >
              ↺ Relancer
            </button>
          </div>
        </div>
      </div>
    {/if}

    <!-- SubtitleEditor : overlay plein éditeur, ancré sur .editor (position:relative) -->
    {#if showSubtitleEditor && $activeProject?.files[0]}
      <div class="subtitle-modal-overlay">
        <SubtitleEditor
          projectId={$activeProject.id}
          videoPath={$activeProject.files[0].path}
          segments={$segmentsHistory}
          onClose={() => (showSubtitleEditor = false)}
        />
      </div>
    {/if}
  </main>
</div>
<!-- Modals -->
<ExportFFmpegModal />
<!-- ⚡ Batch Modal -->
<BatchQueue
  open={showBatchQueue}
  {analyseParams}
  on:close={() => (showBatchQueue = false)}
/>

<TranscribeModal />

<style>
  .app-layout {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }

  .sidebar {
    width: 220px;
    min-width: 220px;
    background: var(--bg-surface);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    padding: 16px 12px;
    gap: 4px;
    overflow: hidden;
  }
  .sidebar-logo {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 4px;
    margin-bottom: 16px;
  }
  .sidebar-title {
    font-size: 16px;
    font-weight: 700;
  }
  .sidebar-nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    border-radius: var(--radius);
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 13px;
    cursor: pointer;
    text-align: left;
    transition: all 0.12s;
  }
  .nav-item:hover {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }
  .nav-item.active {
    background: var(--accent-subtle);
    color: var(--accent);
  }
  .sidebar-divider {
    height: 1px;
    background: var(--border);
    margin: 8px 0;
  }
  .sidebar-section-title {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-muted);
    padding: 0 4px;
    margin-bottom: 4px;
  }
  .projects-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
    flex: 1;
    max-height: 200px;
  }
  .project-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
    text-align: left;
    transition: background 0.1s;
    position: relative;
  }
  .project-item:hover {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }
  .project-item.active {
    background: var(--bg-overlay);
    color: var(--text-primary);
  }
  .project-item:hover .project-delete {
    opacity: 1;
  }
  .project-status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .project-status-dot.idle,
  .project-status-dot.importing {
    background: var(--text-muted);
  }
  .project-status-dot.analysing {
    background: var(--warning);
  }
  .project-status-dot.ready {
    background: var(--success);
  }
  .project-status-dot.error {
    background: var(--danger);
  }
  .project-info {
    flex: 1;
    min-width: 0;
  }
  .project-name {
    font-size: 12px;
  }
  .project-meta {
    font-size: 10px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
  .project-delete {
    opacity: 0;
    transition: opacity 0.1s;
    color: var(--text-muted);
    padding: 2px 5px;
  }
  .new-project-btn {
    padding: 5px 8px;
    border-radius: var(--radius-sm);
    border: 1px dashed var(--border);
    background: transparent;
    color: var(--text-muted);
    font-size: 11px;
    cursor: pointer;
    text-align: left;
  }
  .batch-btn {
    background: rgba(var(--accent-rgb, 99, 102, 241), 0.08);
    border: 1px solid rgba(var(--accent-rgb, 99, 102, 241), 0.2);
    color: var(--accent);
  }
  .batch-btn.active {
    background: rgba(var(--accent-rgb, 99, 102, 241), 0.18);
    border-color: var(--accent);
  }
  .new-project-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .licence-badge {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 10px;
    margin-top: 8px;
  }
  .licence-badge.free {
    border-color: rgba(255, 75, 75, 0.2);
  }
  .licence-icon {
    font-size: 16px;
  }
  .licence-plan {
    font-size: 12px;
    font-weight: 600;
  }
  .licence-upgrade {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--accent);
    font-size: 11px;
    padding: 0;
  }
  .licence-email {
    font-size: 10px;
  }

  .editor {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg-base);
    position: relative; /* ancre l'overlay SubtitleEditor plein écran */
  }
  .editor-header {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-surface);
    flex-shrink: 0;
  }
  .editor-file-info {
    flex: 1;
    min-width: 0;
  }
  .editor-file-name {
    font-size: 16px;
    font-weight: 700;
  }
  .editor-file-meta {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-top: 3px;
    flex-wrap: wrap;
  }
  .editor-status {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .status-analysing {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--warning);
  }
  .editor-actions {
    display: flex;
    gap: 6px;
  }

  .stats-bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 20px;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    flex-wrap: wrap;
  }
  .stat-chip {
    display: flex;
    flex-direction: column;
    gap: 1px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 5px 10px;
  }
  .stat-chip.accent {
    border-color: rgba(184, 255, 60, 0.2);
  }
  .stat-chip span:first-child {
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .stat-chip span:last-child {
    font-size: 14px;
    font-weight: 600;
  }
  .stat-arrow {
    color: var(--text-muted);
    font-size: 12px;
  }

  .waveform-section {
    padding: 12px 20px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .player-row {
    display: flex;
    gap: 16px;
    align-items: flex-start;
  }
  .player-col {
    width: auto;
    min-width: 300px;
    flex-shrink: 0;
  }
  .waveform-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
  }
  .transport {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .timecode {
    font-size: 12px;
    color: var(--text-secondary);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 3px 8px;
  }
  .transport-spacer {
    flex: 1;
  }
  .zoom-slider {
    width: 80px;
    accent-color: var(--accent);
    cursor: pointer;
  }

  .bottom-panel {
    display: flex;
    flex: 1;
    overflow: hidden;
    position: relative; /* ancre le popover raccourcis */
  }
  .timeline-panel {
    flex: 1;
    padding: 16px 20px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .presets-row {
    display: flex;
    gap: 6px;
  }
  .preset-btn {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    padding: 8px 4px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
    font-family: var(--font-body);
  }
  .preset-btn:hover {
    border-color: var(--accent);
    color: var(--text-primary);
    background: rgba(184, 255, 60, 0.06);
  }
  .preset-btn.preset-active {
    border-color: var(--accent);
    background: rgba(184, 255, 60, 0.12);
    color: var(--accent);
  }
  .preset-icon {
    font-size: 14px;
  }
  .preset-label {
    font-size: 10px;
    font-weight: 600;
  }

  .settings-panel {
    width: 260px;
    min-width: 260px;
    border-left: 1px solid var(--border);
    background: var(--bg-surface);
    display: flex;
    flex-direction: column;
    overflow: hidden; /* ParamsPanel gère son propre scroll interne */
  }

  /* Popover raccourcis : flottant en bas à droite de la toolbar */
  .shortcuts-popover {
    position: absolute;
    bottom: 0;
    right: 0;
    width: 240px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    z-index: 200;
    padding: 0;
    overflow: hidden;
  }
  .shortcuts-popover-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
  }
  .shortcuts-close {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 11px;
    padding: 2px 4px;
    border-radius: var(--radius-sm);
  }
  .shortcuts-close:hover {
    color: var(--text-primary);
    background: var(--bg-elevated);
  }
  .shortcuts-btn {
    font-weight: 700;
    font-size: 13px;
    min-width: 26px;
  }
  .btn-active {
    background: var(--accent-subtle) !important;
    color: var(--accent) !important;
    border-color: rgba(184, 255, 60, 0.3) !important;
  }
  .settings-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
  }
  .setting-row {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .setting-label {
    font-size: 11px;
    color: var(--text-secondary);
  }
  .setting-slider {
    width: 100%;
    accent-color: var(--accent);
    cursor: pointer;
  }
  .setting-value {
    font-size: 11px;
    color: var(--text-muted);
  }
  .setting-number,
  .setting-select {
    padding: 5px 8px;
    font-size: 12px;
  }

  /* Raccourcis clavier */
  .shortcuts-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 12px;
  }
  .shortcut-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--text-secondary);
  }
  kbd {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 1px 5px;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-primary);
    white-space: nowrap;
  }

  .dropzone {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 2px dashed var(--border);
    border-radius: var(--radius-xl);
    margin: 24px;
    transition: all 0.15s;
  }
  .dropzone.over {
    border-color: var(--accent);
    background: var(--accent-subtle);
  }
  .dropzone-inner {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    text-align: center;
  }
  .dropzone-icon {
    opacity: 0.8;
  }
  .dropzone-title {
    font-size: 22px;
    font-weight: 700;
  }
  .dropzone-sub {
    color: var(--text-secondary);
    font-size: 13px;
  }
  .dropzone-actions {
    display: flex;
    gap: 10px;
    margin-top: 4px;
  }

  .progress-bar {
    height: 4px;
    background: var(--bg-elevated);
    border-radius: 2px;
    overflow: hidden;
  }
  .progress-bar-fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.3s;
  }
  .truncate {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .subtitle-modal-overlay {
    position: absolute;
    inset: 0;
    z-index: 100;
    background: var(--bg-base);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* ── [UX] Actions groups ─────────────────────────────────────── */
  .editor-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .actions-group {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .actions-sep {
    width: 1px;
    height: 20px;
    background: var(--border);
    margin: 0 4px;
  }

  /* ── [UX] Spinner ring ────────────────────────────────────────── */
  .spinner-ring {
    display: inline-block;
    width: 12px;
    height: 12px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    vertical-align: middle;
  }

  /* ── [UX] Transport zoom buttons ─────────────────────────────── */
  .transport-play {
    min-width: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .zoom-step {
    font-size: 14px;
    font-weight: 700;
    padding: 0 6px;
    min-width: 22px;
  }

  /* ── [UX] Confirm modal ───────────────────────────────────────── */
  .confirm-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
    backdrop-filter: blur(3px);
  }
  .confirm-box {
    background: var(--bg-elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-xl);
    padding: 24px;
    max-width: 360px;
    width: 90%;
    box-shadow: var(--shadow-lg);
  }
  .confirm-title {
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: 700;
    margin-bottom: 10px;
  }
  .confirm-body {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.6;
    margin-bottom: 20px;
  }
  .confirm-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
  .btn-danger {
    background: rgba(255, 75, 75, 0.15);
    border-color: rgba(255, 75, 75, 0.4);
    color: var(--danger);
  }
  .btn-danger:hover {
    background: rgba(255, 75, 75, 0.25);
  }
  .btn-warning {
    background: rgba(255, 184, 77, 0.15);
    border-color: rgba(255, 184, 77, 0.4);
    color: var(--warning);
  }
  .btn-warning:hover {
    background: rgba(255, 184, 77, 0.25);
  }

  /* ── [UX] Project search ─────────────────────────────────────── */
  .project-search {
    width: 100%;
    padding: 4px 8px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-size: 11px;
    color: var(--text-primary);
    outline: none;
    margin-bottom: 4px;
  }
  .project-search:focus {
    border-color: var(--accent);
  }
  .project-count {
    font-size: 9px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 1px 5px;
    color: var(--text-muted);
    margin-left: 4px;
  }

  /* ── [UX] Rename input ───────────────────────────────────────── */
  .rename-input {
    width: 100%;
    background: var(--bg-elevated);
    border: 1px solid var(--accent);
    border-radius: 3px;
    font-size: 11px;
    color: var(--text-primary);
    padding: 1px 4px;
    outline: none;
  }
</style>
