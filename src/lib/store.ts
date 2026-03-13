// ============================================================
// FICHIER : trimlab/src/lib/store.ts
// ROLE    : Etat global reactif - ZERO donnee mockee
//           Toutes les donnees viennent de SQLite via commands.ts
//           L'UI s'abonne aux stores et se met a jour automatiquement
// ============================================================

import { writable, derived } from 'svelte/store'

// ─── Types (mirroir exact des structs Rust) ───────────────────────────────────

export type View = 'landing' | 'app' | 'dashboard'
export type LicenceStatus = 'free' | 'monthly' | 'annual' | 'lifetime' | 'expired'
export type ProjectStatus = 'idle' | 'importing' | 'analysing' | 'ready' | 'exporting' | 'done' | 'error'
export type SegmentType = 'keep' | 'silence' | 'filler' | 'cut'
export type AiMode = 'lite' | 'fast' | 'quality'

export interface ProjectSettings {
  silence_threshold: number
  silence_min_duration: number
  filler_words: string[]
  padding_before: number
  padding_after: number
  ai_mode: AiMode
  language: string
}

export interface MediaFile {
  id: string
  project_id: string
  name: string
  path: string
  duration: number
  size: number
  media_type: string
  has_video: boolean
  has_audio: boolean
  fps?: number
  codec?: string
  sample_rate?: number
  added_at: string
}

export interface Segment {
  id: string
  project_id: string
  start_time: number
  end_time: number
  seg_type: SegmentType
  confidence: number
  label?: string
}

export interface ProcessingStats {
  original_duration: number
  trimmed_duration: number
  silences_removed: number
  fillers_removed: number
  space_saved: number
  processing_time: number
  accuracy: number
}

export interface Project {
  id: string
  name: string
  status: ProjectStatus
  progress: number
  export_format: string
  detection_mode: string
  created_at: string
  updated_at: string
  settings: ProjectSettings
  files: MediaFile[]
  stats?: ProcessingStats
}

export interface LicenceInfo {
  status: LicenceStatus
  email?: string
  activated_at?: string
  expires_at?: string
  machine_id?: string
  activations: number
  max_activations: number
}

export interface Notification {
  id: string
  type: 'info' | 'success' | 'warning' | 'error'
  title: string
  message?: string
  duration?: number
  action?: { label: string; fn: () => void }
}

export interface DashboardStats {
  total_projects: number
  total_files: number
  total_time_saved: number
  total_exports: number
  avg_accuracy: number
}

// ─── Stores reactifs ─────────────────────────────────────────────────────────
// Aucune valeur mockee : tout part de zero ou d'un defaut neutre

export const currentView = writable<View>('landing')

// Licence : etat par defaut "free" jusqu'au chargement depuis SQLite
export const licence = writable<LicenceInfo>({
  status: 'free',
  activations: 0,
  max_activations: 2,
})

// Projets : tableau vide, rempli apres list_projects() depuis Rust
export const projects = writable<Project[]>([])
export const activeProjectId = writable<string | null>(null)

// Segments du projet actif : charges depuis SQLite a chaque changement de projet
export const segments = writable<Segment[]>([])

// Notifications (UI only, pas persistees)
export const notifications = writable<Notification[]>([])

// Lecture / waveform
export const waveformData = writable<number[]>([])
export const playhead = writable<number>(0)
export const isPlaying = writable<boolean>(false)
export const zoom = writable<number>(1)
export const isAnalysing = writable<boolean>(false)
export const isImporting = writable<boolean>(false)

// Modales
export const showLicenceModal = writable<boolean>(false)
export const showExportModal = writable<boolean>(false)
export const showExportFFmpegModal = writable<boolean>(false)
export const showTranscribeModal = writable<boolean>(false)
export const showSettingsModal = writable<boolean>(false)

// Dashboard : charge depuis SQLite via get_dashboard_stats()
export const dashboardStats = writable<DashboardStats | null>(null)

// ─── Derived stores ──────────────────────────────────────────────────────────

export const activeProject = derived(
  [projects, activeProjectId],
  ([$projects, $id]: [Project[], string | null]) =>
    $projects.find((p: Project) => p.id === $id) ?? null
)

// Export autorisé pour les plans payants (monthly, annual, lifetime)
// Les utilisateurs free peuvent exporter en XML uniquement
export const canExport = derived(
  licence,
  ($l: LicenceInfo) => $l.status === 'lifetime'
)

const planLabels: Record<LicenceStatus, string> = {
  free: 'Gratuit',
  monthly: 'Mensuel',
  annual: 'Annuel',
  lifetime: 'A vie',
  expired: 'Expire',
}

export const licencePlanLabel = derived(
  licence,
  ($l: LicenceInfo) => planLabels[$l.status]
)

// Segments du projet actif filtres par type
export const keepSegments = derived(
  segments,
  ($segs: Segment[]) => $segs.filter((s: Segment) => s.seg_type === 'keep')
)

export const silenceSegments = derived(
  segments,
  ($segs: Segment[]) => $segs.filter((s: Segment) => s.seg_type === 'silence')
)

// ─── Actions UI ──────────────────────────────────────────────────────────────

let _notifId = 0
export function dismissNotif(id: string): void {
  notifications.update((ns: Notification[]) => ns.filter((x: Notification) => x.id !== id))
}

export function notify(n: Omit<Notification, 'id'>): string {
  const id = `notif-${++_notifId}`
  notifications.update((ns: Notification[]) => [...ns, { ...n, id }])
  const duration = n.duration ?? 4000
  if (duration > 0) {
    setTimeout(() => {
      notifications.update((ns: Notification[]) => ns.filter((x: Notification) => x.id !== id))
    }, duration)
  }
  return id
}

// Met a jour un projet dans le store local (apres sync SQLite)
export function updateProjectInStore(updated: Project): void {
  projects.update((ps: Project[]) =>
    ps.map((p: Project) => p.id === updated.id ? updated : p)
  )
}

// Retire un projet du store local
export function removeProjectFromStore(id: string): void {
  projects.update((ps: Project[]) => ps.filter((p: Project) => p.id !== id))
  activeProjectId.update((cur: string | null) => cur === id ? null : cur)
}

// ── Subtitle blocks cache (survit aux remontages du composant) ──────────────
export const subtitleBlocksCache = writable<Record<string, any[]>>({})