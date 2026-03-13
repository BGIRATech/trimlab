// FICHIER : trimlab/src/lib/commands.ts
// ============================================================

import { invoke as tauriInvoke } from '@tauri-apps/api/core'
import { open as tauriOpen } from '@tauri-apps/plugin-dialog'
import type {
  Project, ProjectSettings, MediaFile, Segment,
  ProcessingStats, LicenceInfo, DashboardStats
} from './store'

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

// CORRECTION : Ajout de <T> après Promise
async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) {
    return await tauriInvoke<T>(cmd, args)
  }
  return browserFallback(cmd, args) as T
}

export const commands = {

  // ─── Projets ────────────────────────────────────────────────

  listProjects: (): Promise<Project[]> =>
    invoke('list_projects'),

  createProject: (name: string): Promise<Project> =>
    invoke('create_project', { name }),

  deleteProject: (id: string): Promise<void> =>
    invoke('delete_project', { id }),

  updateProjectStatus: (id: string, status: string, progress: number): Promise<void> =>
    invoke('update_project_status', { id, status, progress }),

  updateProjectSettings: (id: string, settings: ProjectSettings): Promise<void> =>
    invoke('update_project_settings', { id, settings }),

  addMediaFile: (projectId: string, file: Omit<MediaFile, 'id' | 'added_at'>): Promise<MediaFile> =>
    invoke('add_media_file', { projectId, file }),

  saveProcessingStats: (projectId: string, stats: ProcessingStats): Promise<void> =>
    invoke('save_processing_stats', { projectId, stats }),

  getDashboardStats: (): Promise<DashboardStats> =>
    invoke('get_dashboard_stats'),

  // ─── Segments ────────────────────────────────────────────────

  listSegments: (projectId: string): Promise<Segment[]> =>
    invoke('list_segments', { projectId }),

  saveSegments: (projectId: string, segments: Segment[]): Promise<void> =>
    invoke('save_segments', { projectId, segments }),

  toggleSegment: (segmentId: string): Promise<Segment> =>
    invoke('toggle_segment', { segmentId }),

  deleteSegment: (segmentId: string): Promise<void> =>
    invoke('delete_segment', { segmentId }),

  analyseAndSave: (
    projectId: string,
    filePath: string,
    thresholdDb: number,
    minDurationMs: number,
    duration: number,
    paddingBefore: number,
    paddingAfter: number,
    minSpeechMs: number,
    aggressiveness: number,
  ): Promise<Segment[]> =>
    invoke('analyse_and_save', {
      projectId, filePath, thresholdDb, minDurationMs, duration,
      paddingBefore, paddingAfter, minSpeechMs, aggressiveness,
    }),

  // ─── Licence ─────────────────────────────────────────────────

  getLicence: (): Promise<LicenceInfo> =>
    invoke('get_licence'),

  validateAndActivateLicence: (key: string): Promise<{
    valid: boolean
    plan?: string
    email?: string
    error?: string
  }> =>
    invoke('validate_and_activate_licence', { key }),

  deactivateLicence: (): Promise<void> =>
    invoke('deactivate_licence'),

  getMachineId: (): Promise<string> =>
    invoke('get_machine_id'),

  // ─── Media (FFmpeg) ───────────────────────────────────────────

  probeMedia: (path: string): Promise<{
    duration: number
    size: number
    has_video: boolean
    has_audio: boolean
    fps?: number
    codec?: string
    sample_rate?: number
  }> =>
    invoke('probe_media', { path }),

  getWaveformData: (path: string, points?: number): Promise<number[]> =>
    invoke('get_waveform_data', { path, points }),

  exportSegments: (projectId: string, format: string, outputDir: string): Promise<{
    success: boolean
    output_path: string
    segments_exported: number
  }> =>
    invoke('export_segments', { projectId, format, outputDir }),

  openFileDialog: async (): Promise<string | null> => {
    if (isTauri) {
      const result = await tauriOpen({
        multiple: false,
        filters: [{ name: 'Media', extensions: ['mp4', 'mov', 'mkv', 'avi', 'webm', 'mp3', 'wav', 'flac', 'm4a', 'aac'] }]
      })
      return typeof result === 'string' ? result : null
    }
    return null
  },

  getFfmpegVersion: (): Promise<string> =>
    invoke('get_ffmpeg_version'),

  // ─── Export FFmpeg direct ─────────────────────────────────────

  exportFfmpeg: (opts: {
    projectId: string
    outputPath: string
    mode: 'copy' | 'reencode'
    format: 'mp4' | 'mkv' | 'mov'
  }): Promise<{
    success: boolean
    outputPath: string
    segmentsExported: number
    durationSaved: number
    error?: string
  }> =>
    invoke('export_ffmpeg', {
      opts: {
        project_id: opts.projectId,
        output_path: opts.outputPath,
        mode: opts.mode,
        format: opts.format,
      }
    }),
  exportSrt: (args: {
    projectId: string
    blocks: Array<{ start: number; end: number; text: string }>
  }): Promise<{ success: boolean; outputPath: string; error?: string }> =>
    invoke('export_srt', {
      args: {
        project_id: args.projectId,
        blocks: args.blocks,
      }
    }),

  exportAss: (args: {
    projectId: string
    blocks: Array<{ start: number; end: number; text: string; words: Array<{ word: string; start: number; end: number }> }>
    style: object
  }): Promise<{ success: boolean; outputPath: string; error?: string }> =>
    invoke('export_ass', {
      args: {
        project_id: args.projectId,
        blocks: args.blocks,
        style: args.style,
      }
    }),

  burnSubtitles: (args: {
    projectId: string
    videoPath: string
    blocks: Array<{ start: number; end: number; text: string; words: Array<{ word: string; start: number; end: number }>; edited?: boolean }>
    style: object
    keepSegments?: Array<[number, number]>
  }): Promise<{ success: boolean; outputPath: string; error?: string }> =>
    invoke('burn_subtitles', {
      args: {
        project_id: args.projectId,
        video_path: args.videoPath,
        blocks: args.blocks,
        style: args.style,
        keep_segments: args.keepSegments ?? [],
      }
    }),

  // ─── Whisper / Transcription ──────────────────────────────────

  checkWhisperAvailable: (): Promise<boolean> =>
    invoke('check_whisper_available'),

  // Nouvelles fonctions
  ensureWhisperModel: (modelName: string): Promise<string> =>
    invoke('ensure_whisper_model', { modelName }),

  getLocalModelPath: (modelName: string): Promise<string> =>
    invoke('get_local_model_path', { modelName }),

  transcribeAndDetect: (config: {
    projectId: string
    modelPath: string
    language: string
    detectFillers: boolean
    fillerWords?: string[]
    detectRepeats: boolean
    repeatWindow: number
    repeatThreshold: number
  }): Promise<{
    success: boolean
    language: string
    segmentsCount: number
    wordsCount: number
    fillersFound: number
    repeatsFound: number
    error?: string
  }> =>
    invoke('transcribe_and_detect', {
      config: {
        project_id: config.projectId,
        model_path: config.modelPath,
        language: config.language,
        detect_fillers: config.detectFillers,
        filler_words: config.fillerWords ?? [],
        detect_repeats: config.detectRepeats,
        repeat_window: config.repeatWindow,
        repeat_threshold: config.repeatThreshold,
      }
    }),

  getTranscriptWords: (projectId: string): Promise<Array<{
    word: string
    start: number
    end: number
    confidence: number
  }>> =>
    invoke('get_transcript_words', { projectId }),

  exportMultiXml: (opts: {
    projectIds: string[]
    outputPath: string
    title: string
  }): Promise<{
    success: boolean
    outputPath: string
    clipsExported: number
    projectsMerged: number
    error?: string
  }> =>
    invoke('export_multi_xml', {
      opts: {
        project_ids: opts.projectIds,
        output_path: opts.outputPath,
        title: opts.title,
      }
    }),

  exportAudio: (opts: {
    projectId: string
    outputPath: string
    format: 'mp3' | 'wav' | 'aac'
    quality: 'low' | 'medium' | 'high'
  }): Promise<{
    success: boolean
    outputPath: string
    segmentsExported: number
    error?: string
  }> =>
    invoke('export_audio', {
      opts: {
        project_id: opts.projectId,
        output_path: opts.outputPath,
        format: opts.format,
        quality: opts.quality,
      }
    }),

  deleteProjectWords: (projectId: string): Promise<void> =>
    invoke('delete_project_words', { projectId }),

  saveSubtitleBlocks: (projectId: string, blocksJson: string): Promise<void> =>
    invoke('save_subtitle_blocks', { projectId, blocksJson }),

  getSubtitleBlocks: (projectId: string): Promise<string | null> =>
    invoke('get_subtitle_blocks', { projectId }),

  transcribeBatch: (jobs: Array<{
    projectId: string
    modelPath: string
    language: string
    detectFillers: boolean
    detectRepeats: boolean
    repeatWindow: number
    repeatThreshold: number
  }>): Promise<number> =>
    invoke('transcribe_batch', {
      jobs: jobs.map(j => ({
        project_id: j.projectId,
        model_path: j.modelPath,
        language: j.language,
        detect_fillers: j.detectFillers,
        detect_repeats: j.detectRepeats,
        repeat_window: j.repeatWindow,
        repeat_threshold: j.repeatThreshold,
      }))
    }),

  // ─── Sync audio externe ───────────────────────────────────────

  syncExternalAudio: (args: {
    projectId: string
    videoPath: string
    audioPath: string
  }): Promise<{
    success: boolean
    offsetSecs: number
    confidence: number
    method: string
    error?: string
  }> =>
    invoke('sync_external_audio', {
      projectId: args.projectId,
      videoPath: args.videoPath,
      audioPath: args.audioPath,
    }),

  getAudioSyncInfo: (projectId: string): Promise<{
    externalAudioPath: string | null
    audioOffset: number
  }> =>
    invoke('get_audio_sync_info', { projectId }),

  removeExternalAudio: (projectId: string): Promise<void> =>
    invoke('remove_external_audio', { projectId }),


  // ─── Analyse batch parallèle ──────────────────────────────────
  //
  // Lance N analyses VAD en parallèle côté Rust (std::thread::spawn).
  // La progression arrive via l'event Tauri "batch-analyse-progress".
  // Retourne le nombre de jobs lancés.
  analyseBatch: (jobs: Array<{
    projectId: string
    filePath: string
    thresholdDb: number
    minDurationMs: number
    duration: number
    paddingBefore: number
    paddingAfter: number
    minSpeechMs: number
    aggressiveness: number
  }>): Promise<number> =>
    invoke('analyse_batch', {
      jobs: jobs.map(j => ({
        project_id: j.projectId,
        file_path: j.filePath,
        threshold_db: j.thresholdDb,
        min_duration_ms: j.minDurationMs,
        duration: j.duration,
        padding_before: j.paddingBefore,
        padding_after: j.paddingAfter,
        min_speech_ms: j.minSpeechMs,
        aggressiveness: j.aggressiveness,
      }))
    }),

  // ─── Conversion source MKV → MP4 ─────────────────────────────

  // [FIX MKV] Convertit la SOURCE complète (pas le montage découpé) en MP4.
  // Nécessaire pour que le XML/EDL généré soit lisible dans Premiere Pro.
  // Utilise stream-copy si possible (rapide), sinon re-encode en H.264.
  convertSourceToMp4: (projectId: string, inputPath: string, outputPath: string): Promise<string> =>
    invoke('convert_source_to_mp4', { projectId, inputPath, outputPath }),

  // [FIX MKV] Met à jour le chemin du fichier source en base après conversion.
  // Le XML suivant référencera le nouveau .mp4.
  updateMediaFilePath: (projectId: string, newPath: string): Promise<void> =>
    invoke('update_media_file_path', { projectId, newPath }),

  saveTextFile: (path: string, content: string): Promise<void> =>
    invoke('save_text_file', { path, content }),

  generateChapters: (projectId: string): Promise<Array<{
    time_edited: number
    time_source: number
    title: string
  }>> =>
    invoke('generate_chapters', { projectId }),
}


// ─── Fallback navigateur ──────────────────────────────────────

function browserFallback(cmd: string, _args?: Record<string, unknown>): unknown {
  switch (cmd) {
    case 'list_projects': return []
    case 'create_project': return null
    case 'delete_project': return undefined
    case 'update_project_status': return undefined
    case 'update_project_settings': return undefined
    case 'add_media_file': return null
    case 'save_processing_stats': return undefined
    case 'get_dashboard_stats': return { total_projects: 0, total_files: 0, total_time_saved: 0, total_exports: 0, avg_accuracy: 0 }
    case 'list_segments': return []
    case 'save_segments': return undefined
    case 'toggle_segment': return null
    case 'delete_segment': return undefined
    case 'analyse_and_save': return []
    case 'get_licence': return { status: 'free', activations: 0, max_activations: 2 }
    case 'validate_and_activate_licence': return { valid: false, error: 'Tauri requis' }
    case 'deactivate_licence': return undefined
    case 'get_machine_id': return 'BROWSER-MODE'
    case 'probe_media': return { duration: 0, size: 0, has_video: false, has_audio: false }
    case 'get_waveform_data': return new Array(2048).fill(0)
    case 'export_segments': return { success: false, output_path: '', segments_exported: 0 }
    case 'open_file_dialog': return null
    case 'get_ffmpeg_version': return 'FFmpeg (non disponible)'
    case 'export_ffmpeg': return { success: false, output_path: '', segments_exported: 0, duration_saved: 0 }
    case 'check_whisper_available': return false
    case 'ensure_whisper_model': return '/fake/path/model.bin'
    case 'get_local_model_path': return '/fake/path/model.bin'
    case 'transcribe_and_detect': return { success: false, language: 'fr', segments_count: 0, words_count: 0, fillers_found: 0, repeats_found: 0, error: 'Tauri requis' }
    case 'get_transcript_words': return []
    case 'export_srt': return { success: false, output_path: '', error: 'Tauri requis' }
    case 'export_ass': return { success: false, output_path: '', error: 'Tauri requis' }
    case 'burn_subtitles': return { success: false, output_path: '', error: 'Tauri requis' }
    case 'export_multi_xml': return { success: false, output_path: '', clips_exported: 0, projects_merged: 0, error: 'Tauri requis' }
    case 'export_audio': return { success: false, output_path: '', segments_exported: 0, error: 'Tauri requis' }
    case 'delete_project_words': return undefined
    // [FIX BUG E] Cases manquants → le default renvoyait null, provoquant
    // un crash dans SubtitleEditor.onMount qui attend string | null proprement
    case 'save_subtitle_blocks': return undefined
    case 'get_subtitle_blocks': return null
    case 'transcribe_batch': return 0
    case 'sync_external_audio': return { success: false, offsetSecs: 0, confidence: 0, method: 'none', error: 'Tauri requis' }
    case 'get_audio_sync_info': return { externalAudioPath: null, audioOffset: 0 }
    case 'remove_external_audio': return undefined
    case 'analyse_batch': return 0
    case 'convert_source_to_mp4': return '/fake/converted.mp4'
    case 'update_media_file_path': return undefined
    case 'generate_chapters': return []
    case 'save_text_file': return undefined
    default: return null
  }
}