// Ambient Store for LOCUS System Tray and Spotlight HUD synchronization (Vanilla TypeScript + React hooks)
import { useState, useEffect } from 'react';

export interface AmbientTelemetryState {
  ramUsageMb: number;
  latencyMs: number;
  tokensSavedPct: number;
  cloudCost: number;
  activeApp: string;
  isSpotlightVisible: boolean;
  isPinned: boolean;
  selectedIntent: string;
  query: string;
}

let currentState: AmbientTelemetryState = {
  ramUsageMb: 38.5,
  latencyMs: 1.8,
  tokensSavedPct: 96.0,
  cloudCost: 0.0,
  activeApp: 'Active Editor',
  isSpotlightVisible: false,
  isPinned: false,
  selectedIntent: 'search',
  query: '',
};

type Listener = (state: AmbientTelemetryState) => void;
const listeners = new Set<Listener>();

function emit() {
  for (const listener of listeners) {
    listener(currentState);
  }
}

export const ambientStore = {
  getState: (): AmbientTelemetryState => currentState,
  
  setSpotlightVisible: (visible: boolean) => {
    currentState = { ...currentState, isSpotlightVisible: visible };
    emit();
  },

  setPinned: (pinned: boolean) => {
    currentState = { ...currentState, isPinned: pinned };
    emit();
  },

  setIntent: (intent: string) => {
    currentState = { ...currentState, selectedIntent: intent };
    emit();
  },

  setQuery: (query: string) => {
    currentState = { ...currentState, query };
    emit();
  },

  updateTelemetry: (telemetry: Partial<AmbientTelemetryState>) => {
    currentState = { ...currentState, ...telemetry };
    emit();
  },

  toggleSpotlight: () => {
    currentState = { ...currentState, isSpotlightVisible: !currentState.isSpotlightVisible };
    emit();
  },

  subscribe: (listener: Listener) => {
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  },
};

export function useAmbientStore(): [AmbientTelemetryState, typeof ambientStore] {
  const [state, setState] = useState<AmbientTelemetryState>(ambientStore.getState());

  useEffect(() => {
    const unsubscribe = ambientStore.subscribe((nextState) => {
      setState(nextState);
    });
    return unsubscribe;
  }, []);

  return [state, ambientStore];
}
