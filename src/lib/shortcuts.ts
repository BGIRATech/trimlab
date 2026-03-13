// FICHIER : src/lib/shortcuts.ts
// ROLE    : Raccourcis clavier globaux pour l'éditeur
//           JKL navigation (standard montage), Espace, trim, undo
// ============================================================

import { get } from 'svelte/store'
import { playhead, isPlaying, zoom } from './store'
// [FIX BUG B] Importer segmentsHistory (source de vérité) au lieu de 'segments'
// qui est un store legacy jamais mis à jour dans App.svelte
import { segmentsHistory } from './history'

export type ShortcutHandler = () => void

interface ShortcutDef {
    key: string
    ctrl?: boolean
    shift?: boolean
    description: string
    handler: ShortcutHandler
}

let registered = false
let seekCallback: ((t: number) => void) | null = null
let undoCallback: (() => void) | null = null
let redoCallback: (() => void) | null = null
let toggleCurrentCallback: (() => void) | null = null

export function initShortcuts(opts: {
    onSeek: (t: number) => void
    onUndo: () => void
    onRedo: () => void
    onToggleCurrent?: () => void
}) {
    seekCallback = opts.onSeek
    undoCallback = opts.onUndo
    redoCallback = opts.onRedo
    toggleCurrentCallback = opts.onToggleCurrent ?? null

    if (registered) return
    registered = true
    window.addEventListener('keydown', handleKey)
}

export function destroyShortcuts() {
    window.removeEventListener('keydown', handleKey)
    registered = false
}

function handleKey(e: KeyboardEvent) {
    // Ignorer si l'utilisateur tape dans un input / textarea
    const target = e.target as HTMLElement
    if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) return

    const ctrl = e.ctrlKey || e.metaKey

    switch (e.key) {
        // ── Lecture ───────────────────────────────────────────────────────────────
        case ' ':
        case 'k':
        case 'K':
            e.preventDefault()
            isPlaying.update(v => !v)
            break

        // ── JKL navigation (standard montage) ────────────────────────────────────
        case 'j':
        case 'J':
            e.preventDefault()
            isPlaying.set(false)
            seek(-5)
            break

        case 'l':
        case 'L':
            e.preventDefault()
            // L double la vitesse si déjà en lecture, sinon play
            if (get(isPlaying)) {
                seek(5)
            } else {
                isPlaying.set(true)
            }
            break

        // ── Navigation frame par frame ────────────────────────────────────────────
        case 'ArrowLeft':
            e.preventDefault()
            isPlaying.set(false)
            seek(e.shiftKey ? -1 : -0.1)
            break

        case 'ArrowRight':
            e.preventDefault()
            isPlaying.set(false)
            seek(e.shiftKey ? 1 : 0.1)
            break

        // ── Navigation par segments ───────────────────────────────────────────────
        case 'ArrowUp':
            e.preventDefault()
            jumpToPrevSegment()
            break

        case 'ArrowDown':
            e.preventDefault()
            jumpToNextSegment()
            break

        // ── Toggle segment actif ─────────────────────────────────────────────────────
        case 'e':
        case 'E':
            e.preventDefault()
            toggleCurrentCallback?.()
            break

        // ── Undo / Redo ────────────────────────────────────────────────────────────
        case 'z':
        case 'Z':
            if (ctrl) {
                e.preventDefault()
                if (e.shiftKey) redoCallback?.()
                else undoCallback?.()
            }
            break

        case 'y':
        case 'Y':
            if (ctrl) {
                e.preventDefault()
                redoCallback?.()
            }
            break

        // ── Zoom ──────────────────────────────────────────────────────────────────
        case '+':
        case '=':
            if (ctrl) { e.preventDefault(); zoom.update(z => Math.min(20, z * 1.2)) }
            break

        case '-':
            if (ctrl) { e.preventDefault(); zoom.update(z => Math.max(1, z / 1.2)) }
            break

        case '0':
            if (ctrl) { e.preventDefault(); zoom.set(1) }
            break

        // ── Retour début ──────────────────────────────────────────────────────────
        case 'Home':
            e.preventDefault()
            isPlaying.set(false)
            playhead.set(0)
            seekCallback?.(0)
            break
    }
}

function seek(delta: number) {
    playhead.update(t => {
        const newT = Math.max(0, t + delta)
        seekCallback?.(newT)
        return newT
    })
}

function jumpToNextSegment() {
    // [FIX BUG B] Utiliser segmentsHistory (données réelles) au lieu du store 'segments' vide
    const segs = get(segmentsHistory)
    const ph = get(playhead)
    const keeps = segs.filter(s => s.seg_type === 'keep')
    const next = keeps.find(s => s.start_time > ph + 0.1)
    if (next) {
        playhead.set(next.start_time)
        seekCallback?.(next.start_time)
    }
}

function jumpToPrevSegment() {
    // [FIX BUG B] Utiliser segmentsHistory (données réelles) au lieu du store 'segments' vide
    const segs = get(segmentsHistory)
    const ph = get(playhead)
    const keeps = segs.filter(s => s.seg_type === 'keep')
    const prev = [...keeps].reverse().find(s => s.end_time < ph - 0.1)
    if (prev) {
        playhead.set(prev.start_time)
        seekCallback?.(prev.start_time)
    }
}

// ── Référence des raccourcis pour l'aide ──────────────────────────────────────
export const SHORTCUTS_HELP: Array<{ keys: string; description: string }> = [
    { keys: 'Espace / K', description: 'Play / Pause' },
    { keys: 'J', description: 'Reculer de 5s' },
    { keys: 'L', description: 'Avancer de 5s / Play' },
    { keys: '← →', description: 'Avancer/Reculer 0.1s' },
    { keys: 'Shift + ← →', description: 'Avancer/Reculer 1s' },
    { keys: '↑ ↓', description: 'Segment précédent/suivant' },
    { keys: 'E', description: 'Couper / Garder segment actif' },
    { keys: 'Ctrl+Z', description: 'Annuler' },
    { keys: 'Ctrl+Shift+Z', description: 'Rétablir' },
    { keys: 'Ctrl + / -', description: 'Zoom +/-' },
    { keys: 'Ctrl+0', description: 'Zoom reset' },
    { keys: 'Home', description: 'Retour au début' },
]