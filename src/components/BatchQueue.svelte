<!--
  FICHIER : trimlab/src/components/BatchQueue.svelte
  ROLE    : Modal de traitement batch — sélection N fichiers → analyse parallèle → export timeline unifiée
-->
<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { save } from "@tauri-apps/plugin-dialog";
  import { projects, notify, updateProjectInStore } from "../lib/store";
  import { commands } from "../lib/commands";

  export let open = false;
  export let analyseParams: {
    thresholdDb: number;
    minDurationMs: number;
    paddingBefore: number;
    paddingAfter: number;
    minSpeechMs: number;
    aggressiveness: number;
  } = {
    thresholdDb: 0,
    minDurationMs: 300,
    paddingBefore: 0.05,
    paddingAfter: 0.12,
    minSpeechMs: 200,
    aggressiveness: 3,
  };

  const dispatch = createEventDispatcher();

  type JobStep = "pending" | "probing" | "analysing" | "done" | "error";
  interface BatchJob {
    filePath: string;
    fileName: string;
    ext: string;
    projectId: string | null;
    step: JobStep;
    progress: number;
    message: string;
    duration: number;
  }

  const PREMIERE_INCOMPATIBLE = ["mkv", "avi", "wmv", "flv", "webm"];
  const SUPPORTED_EXTS = [
    "mp4",
    "mov",
    "mkv",
    "avi",
    "webm",
    "mp3",
    "wav",
    "flac",
    "m4a",
    "aac",
  ];

  let jobs: BatchJob[] = [];
  let isRunning = false;
  let phase: "config" | "running" | "done" = "config";
  let unlisten: UnlistenFn | null = null;

  $: doneCount = jobs.filter((j) => j.step === "done").length;
  $: errorCount = jobs.filter((j) => j.step === "error").length;
  $: pendingCount = jobs.filter(
    (j) => j.step === "pending" || j.step === "error",
  ).length;
  $: allDone = jobs.length > 0 && doneCount + errorCount === jobs.length;
  $: doneIds = jobs
    .filter((j) => j.step === "done" && j.projectId)
    .map((j) => j.projectId as string);

  // Incompatible = fichier dont le chemin ACTUEL en store est encore MKV/AVI/etc.
  // Si l'utilisateur a converti depuis l'éditeur, le store est mis à jour → plus incompatible.
  $: incompatible = jobs.filter((j) => {
    if (j.step !== "done") return false;
    // Chemin actuel dans le store (après éventuelle conversion)
    const proj = $projects.find((p) => p.id === j.projectId);
    const currentPath = proj?.files?.[0]?.path ?? j.filePath;
    const currentExt = currentPath.split(".").pop()?.toLowerCase() ?? j.ext;
    return PREMIERE_INCOMPATIBLE.includes(currentExt);
  });
  $: hasIncompat = incompatible.length > 0;

  onMount(async () => {
    unlisten = await listen<{
      project_id: string;
      step: string;
      progress: number;
      message: string;
    }>("batch-analyse-progress", (e) => {
      const { project_id, step, progress, message } = e.payload;
      jobs = jobs.map((j) =>
        j.projectId === project_id
          ? { ...j, step: step as JobStep, progress, message }
          : j,
      );
      if (step === "done") {
        const proj = $projects.find((p) => p.id === project_id);
        if (proj)
          updateProjectInStore({ ...proj, status: "ready", progress: 100 });
      }
      if (
        jobs.length > 0 &&
        jobs.every((j) => j.step === "done" || j.step === "error")
      ) {
        phase = "done";
        isRunning = false;
      }
    });
  });
  onDestroy(() => {
    unlisten?.();
  });

  function addFilePaths(paths: string[]) {
    const valid = paths.filter((p) =>
      SUPPORTED_EXTS.includes(p.split(".").pop()?.toLowerCase() ?? ""),
    );
    if (valid.length === 0) {
      notify({
        type: "warning",
        title: "Format non supporté",
        message: "Sélectionne des fichiers vidéo ou audio.",
      });
      return;
    }
    const existing = new Set(jobs.map((j) => j.filePath));
    const newJobs: BatchJob[] = valid
      .filter((p) => !existing.has(p))
      .map((p) => ({
        filePath: p,
        fileName: p.split(/[/\\]/).pop() ?? p,
        ext: p.split(".").pop()?.toLowerCase() ?? "",
        projectId: null,
        step: "pending" as JobStep,
        progress: 0,
        message: "En attente",
        duration: 0,
      }));
    if (newJobs.length === 0) {
      notify({
        type: "info",
        title: "Déjà dans la liste",
        message: "Ces fichiers sont déjà présents.",
      });
      return;
    }
    jobs = [...jobs, ...newJobs];
  }

  async function pickFiles() {
    if (isRunning) return;
    try {
      const { open: dialogOpen } = await import("@tauri-apps/plugin-dialog");
      const result = await dialogOpen({
        multiple: true,
        filters: [{ name: "Médias", extensions: SUPPORTED_EXTS }],
      });
      if (!result) return;
      addFilePaths(Array.isArray(result) ? result : [result]);
    } catch {
      notify({
        type: "warning",
        title: "Impossible d'ouvrir le sélecteur",
        message: "",
      });
    }
  }

  async function launchBatch() {
    if (isRunning || pendingCount === 0) return;
    isRunning = true;
    phase = "running";
    const toProcess = jobs.filter(
      (j) => j.step === "pending" || j.step === "error",
    );
    const prepared: BatchJob[] = [];

    for (const job of toProcess) {
      jobs = jobs.map((j) =>
        j.filePath === job.filePath
          ? { ...j, step: "probing", message: "Probe…" }
          : j,
      );
      try {
        const info = await commands.probeMedia(job.filePath);
        const name =
          job.filePath
            .split(/[/\\]/)
            .pop()
            ?.replace(/\.[^.]+$/, "") ?? "Clip";
        const proj = await commands.createProject(name);
        const mf = await commands.addMediaFile(proj.id, {
          project_id: proj.id,
          name: job.fileName,
          path: job.filePath,
          duration: info.duration,
          size: info.size,
          media_type: info.has_video ? "video" : "audio",
          has_video: info.has_video,
          has_audio: info.has_audio,
          fps: info.fps,
          codec: info.codec,
          sample_rate: info.sample_rate,
        });
        await commands.updateProjectStatus(proj.id, "analysing", 0);
        projects.update((ps) => [
          ...ps,
          { ...proj, status: "analysing", progress: 0, files: [mf] },
        ]);
        jobs = jobs.map((j) =>
          j.filePath === job.filePath
            ? {
                ...j,
                projectId: proj.id,
                step: "analysing",
                progress: 5,
                message: "Analyse en cours…",
                duration: info.duration,
              }
            : j,
        );
        prepared.push({ ...job, projectId: proj.id, duration: info.duration });
      } catch (e) {
        jobs = jobs.map((j) =>
          j.filePath === job.filePath
            ? { ...j, step: "error", message: String(e) }
            : j,
        );
      }
    }

    if (prepared.length === 0) {
      isRunning = false;
      phase = "config";
      return;
    }
    try {
      await commands.analyseBatch(
        prepared
          .filter((j) => j.projectId !== null)
          .map((j) => ({
            projectId: j.projectId!,
            filePath: j.filePath,
            thresholdDb: analyseParams.thresholdDb,
            minDurationMs: analyseParams.minDurationMs,
            duration: j.duration,
            paddingBefore: analyseParams.paddingBefore,
            paddingAfter: analyseParams.paddingAfter,
            minSpeechMs: analyseParams.minSpeechMs,
            aggressiveness: analyseParams.aggressiveness,
          })),
      );
    } catch (e) {
      notify({ type: "error", title: "Erreur batch", message: String(e) });
      isRunning = false;
      phase = "config";
    }
  }

  async function exportTimeline() {
    if (doneIds.length < 1) return;
    const outputPath = await save({
      title: "Enregistrer la timeline unifiée",
      defaultPath: "timeline_batch.xml",
      filters: [{ name: "FCP7 XML", extensions: ["xml"] }],
    }).catch(() => null);
    if (!outputPath) return;
    try {
      const r = await commands.exportMultiXml({
        projectIds: doneIds,
        outputPath,
        title: "Timeline TrimLab",
      });
      if (r.success) {
        notify({
          type: "success",
          title: "Timeline exportée ✓",
          message: `${r.clipsExported} clips → ${outputPath.split(/[/\\]/).pop()}`,
          duration: 8000,
        });
      } else {
        notify({
          type: "error",
          title: "Erreur export",
          message: r.error ?? "Erreur inconnue",
        });
      }
    } catch (e) {
      notify({ type: "error", title: "Erreur export", message: String(e) });
    }
  }

  function removeJob(fp: string) {
    if (!isRunning) jobs = jobs.filter((j) => j.filePath !== fp);
  }
  function reset() {
    if (!isRunning) {
      jobs = [];
      phase = "config";
    }
  }
  function close() {
    if (!isRunning) dispatch("close");
  }
</script>

{#if open}
  <div class="backdrop" role="presentation">
    <button class="backdrop-btn" on:click={close} aria-label="Fermer"></button>
    <div class="modal" role="dialog" aria-modal="true">
      <!-- Header -->
      <div class="modal-header">
        <div>
          <h2 class="font-display">⚡ Multi-clips</h2>
          <p class="text-muted" style="font-size:13px;margin-top:3px;">
            Analyse en parallèle · Export timeline unifiée
          </p>
        </div>
        <button
          class="btn btn-ghost btn-sm"
          on:click={close}
          disabled={isRunning}>✕</button
        >
      </div>

      <!-- Body -->
      <div class="modal-body">
        {#if jobs.length === 0}
          <div class="empty-state">
            <div class="empty-icon">📂</div>
            <p class="empty-title">Aucun fichier sélectionné</p>
            <p class="empty-hint">
              Sélectionne plusieurs fichiers vidéo ou audio.<br />Ils seront
              analysés simultanément.
            </p>
            <button class="btn btn-primary" on:click={pickFiles}
              >Sélectionner des fichiers</button
            >
          </div>
        {:else}
          <div class="job-list">
            {#each jobs as job (job.filePath)}
              <div
                class="job-row"
                class:s-done={job.step === "done"}
                class:s-error={job.step === "error"}
                class:s-run={job.step === "analysing" || job.step === "probing"}
              >
                <span class="job-icon">
                  {#if job.step === "analysing" || job.step === "probing"}<span
                      class="spinner-mini"
                    ></span>
                  {:else if job.step === "done"}✓
                  {:else if job.step === "error"}✕
                  {:else}○{/if}
                </span>
                <div class="job-body">
                  <div class="job-top">
                    <span class="job-name" title={job.filePath}
                      >{job.fileName}</span
                    >
                    {#if PREMIERE_INCOMPATIBLE.includes(job.ext)}
                      <span
                        class="badge-incompat"
                        title="Premiere Pro ne supporte pas ce format — ouvre le projet et convertis la source en MP4 avant d'exporter la timeline"
                        >⚠ .{job.ext.toUpperCase()}</span
                      >
                    {/if}
                  </div>
                  {#if job.step !== "pending"}
                    <div class="job-bar">
                      <div
                        class="job-bar-fill s-{job.step}"
                        style="width:{job.step === 'done'
                          ? 100
                          : job.progress}%"
                      ></div>
                    </div>
                  {/if}
                  <div class="job-msg">{job.message}</div>
                </div>
                {#if !isRunning}
                  <button
                    class="job-remove"
                    on:click={() => removeJob(job.filePath)}
                    title="Retirer">✕</button
                  >
                {/if}
              </div>
            {/each}
          </div>

          {#if phase === "running" || phase === "done"}
            <div class="global-prog">
              <div class="global-bar">
                <div
                  class="global-fill"
                  class:complete={allDone && errorCount === 0}
                  style="width:{jobs.length > 0
                    ? Math.round(((doneCount + errorCount) / jobs.length) * 100)
                    : 0}%"
                ></div>
              </div>
              <span class="global-lbl">
                {doneCount}/{jobs.length} terminés
                {#if errorCount > 0}<span class="c-danger">
                    · {errorCount} erreur{errorCount > 1 ? "s" : ""}</span
                  >{/if}
              </span>
            </div>
          {/if}

          {#if phase === "done" && hasIncompat}
            <div class="mkv-warn">
              <span>⚠</span>
              <div>
                <strong
                  >{incompatible.length} fichier{incompatible.length > 1
                    ? "s"
                    : ""} encore incompatible{incompatible.length > 1
                    ? "s"
                    : ""} avec Premiere Pro</strong
                >
                <span>
                  {incompatible
                    .map((j) => {
                      const proj = $projects.find((p) => p.id === j.projectId);
                      return (
                        proj?.files?.[0]?.path?.split(/[/\\]/).pop() ??
                        j.fileName
                      );
                    })
                    .join(", ")} — ouvre chaque projet dans l'éditeur et clique sur
                  «&nbsp;Convertir en MP4&nbsp;».
                </span>
              </div>
            </div>
          {/if}
        {/if}
      </div>

      <!-- Footer -->
      <div class="modal-footer">
        {#if jobs.length > 0 && !isRunning}
          <button class="btn btn-ghost btn-sm" on:click={reset}>Vider</button>
        {/if}
        {#if jobs.length > 0 && phase !== "running"}
          <button
            class="btn btn-ghost btn-sm"
            on:click={pickFiles}
            disabled={isRunning}>+ Ajouter</button
          >
        {/if}
        <div style="flex:1"></div>
        {#if phase === "done" && doneCount >= 1}
          <button
            class="btn btn-success"
            disabled={hasIncompat}
            title={hasIncompat
              ? "Convertissez les fichiers incompatibles en MP4 depuis l'éditeur"
              : ""}
            on:click={exportTimeline}
          >
            📋 Exporter timeline ({doneCount} clip{doneCount > 1 ? "s" : ""})
          </button>
          <button class="btn btn-primary" on:click={close}
            >Voir les projets</button
          >
        {:else if phase !== "running"}
          {#if jobs.length === 0}
            <button class="btn btn-ghost" on:click={close}>Annuler</button>
          {:else}
            <button
              class="btn btn-primary"
              disabled={pendingCount === 0}
              on:click={launchBatch}
            >
              ⚡ Analyser {pendingCount} clip{pendingCount > 1 ? "s" : ""}
            </button>
          {/if}
        {:else}
          <button class="btn btn-primary" disabled
            ><span class="spinner-mini"></span> Analyse en cours…</button
          >
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    z-index: 200;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .backdrop-btn {
    position: absolute;
    inset: 0;
    background: none;
    border: none;
    cursor: default;
  }
  .modal {
    position: relative;
    z-index: 1;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    width: min(620px, 92vw);
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.4);
  }
  .modal-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 18px 22px 14px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .modal-header h2 {
    font-size: 16px;
    margin: 0;
  }
  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px 22px;
    min-height: 100px;
  }
  .modal-footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 22px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 28px 16px;
    gap: 10px;
  }
  .empty-icon {
    font-size: 34px;
    opacity: 0.4;
  }
  .empty-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }
  .empty-hint {
    font-size: 12px;
    color: var(--text-muted);
    margin: 0;
    line-height: 1.6;
  }

  .job-list {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .job-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 11px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    transition: border-color 0.15s;
  }
  .job-row.s-done {
    border-color: rgba(34, 197, 94, 0.3);
  }
  .job-row.s-error {
    border-color: rgba(239, 68, 68, 0.3);
  }
  .job-row.s-run {
    border-color: rgba(99, 102, 241, 0.35);
  }

  .job-icon {
    width: 16px;
    text-align: center;
    font-size: 12px;
    flex-shrink: 0;
    color: var(--text-muted);
  }
  .job-row.s-done .job-icon {
    color: var(--success, #22c55e);
    font-weight: 700;
  }
  .job-row.s-error .job-icon {
    color: var(--danger, #ef4444);
    font-weight: 700;
  }

  .job-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .job-top {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .job-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }
  .badge-incompat {
    flex-shrink: 0;
    font-size: 10px;
    font-weight: 600;
    background: rgba(245, 158, 11, 0.1);
    color: var(--warning, #f59e0b);
    border: 1px solid rgba(245, 158, 11, 0.3);
    border-radius: 3px;
    padding: 1px 5px;
    cursor: help;
  }

  .job-bar {
    height: 3px;
    background: var(--border);
    border-radius: 2px;
    overflow: hidden;
  }
  .job-bar-fill {
    height: 100%;
    border-radius: 2px;
    background: var(--accent);
    transition: width 0.3s ease;
  }
  .job-bar-fill.s-done {
    background: var(--success, #22c55e);
  }
  .job-bar-fill.s-error {
    background: var(--danger, #ef4444);
  }
  .job-msg {
    font-size: 10px;
    color: var(--text-muted);
  }

  .job-remove {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 11px;
    padding: 2px 4px;
    border-radius: 3px;
    opacity: 0.5;
    flex-shrink: 0;
    transition: opacity 0.1s;
  }
  .job-remove:hover {
    opacity: 1;
    color: var(--danger, #ef4444);
  }

  .global-prog {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }
  .global-bar {
    flex: 1;
    height: 4px;
    background: var(--border);
    border-radius: 2px;
    overflow: hidden;
  }
  .global-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
    transition: width 0.4s ease;
  }
  .global-fill.complete {
    background: var(--success, #22c55e);
  }
  .global-lbl {
    font-size: 11px;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .mkv-warn {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    margin-top: 10px;
    padding: 10px 12px;
    background: rgba(239, 68, 68, 0.07);
    border: 1px solid rgba(239, 68, 68, 0.25);
    border-radius: var(--radius);
    font-size: 12px;
  }
  .mkv-warn strong {
    display: block;
    color: var(--danger, #ef4444);
    margin-bottom: 2px;
  }
  .mkv-warn span {
    color: var(--text-muted);
    line-height: 1.5;
  }

  .btn-success {
    background: rgba(34, 197, 94, 0.12);
    color: var(--success, #22c55e);
    border: 1px solid rgba(34, 197, 94, 0.3);
  }
  .btn-success:hover:not(:disabled) {
    background: rgba(34, 197, 94, 0.22);
  }
  .btn-success:disabled {
    opacity: 0.4;
    cursor: not-allowed;
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
  .c-danger {
    color: var(--danger, #ef4444);
  }
</style>
