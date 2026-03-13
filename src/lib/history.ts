// ============================================================
// FICHIER : src/lib/history.ts
// ROLE    : Undo/Redo store wrapper pour les segments
//           Writable avec pile d'historique (50 états max)
// ============================================================

import { writable, get } from 'svelte/store'
import type { Segment } from './store'

const MAX_HISTORY = 50

interface HistoryState {
    past: Segment[][]
    present: Segment[]
    future: Segment[][]
}

function createUndoableSegments() {
    const state: HistoryState = {
        past: [],
        present: [],
        future: [],
    }

    const { subscribe, set: _set } = writable<Segment[]>([])

    function set(newSegs: Segment[]) {
        // Push present → past
        if (state.present.length > 0) {
            state.past.push(state.present)
            if (state.past.length > MAX_HISTORY) state.past.shift()
        }
        state.present = newSegs
        state.future = []
        _set(newSegs)
    }

    // [AJOUT] Méthode update manquante
    function update(fn: (s: Segment[]) => Segment[]) {
        const newValue = fn(state.present);
        set(newValue);
    }

    function undo() {
        if (state.past.length === 0) return false
        state.future.unshift(state.present)
        state.present = state.past.pop()!
        _set(state.present)
        return true
    }

    function redo() {
        if (state.future.length === 0) return false
        state.past.push(state.present)
        state.present = state.future.shift()!
        _set(state.present)
        return true
    }

    function init(segs: Segment[]) {
        state.past = []
        state.present = segs
        state.future = []
        _set(segs)
    }

    function canUndo() { return state.past.length > 0 }
    function canRedo() { return state.future.length > 0 }

    return { subscribe, set, update, undo, redo, init, canUndo, canRedo }
}

export const segmentsHistory = createUndoableSegments()

