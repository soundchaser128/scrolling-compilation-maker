import type { ClipCandidate } from '#lib/types.js';
import { defaultTiming, type TimingSettings } from './timing.js';

interface WorkflowData {
  clips: ClipCandidate[] | null;
  selected: string[];
  ranges: Record<string, [number, number]>;
  jobId: string | null;
  timing: TimingSettings;
}

const empty = (): WorkflowData => ({
  clips: null,
  selected: [],
  ranges: {},
  jobId: null,
  timing: { ...defaultTiming }
});

function restore(): WorkflowData {
  try {
    const saved = sessionStorage.getItem('workflow');
    if (!saved) return empty();
    const data = JSON.parse(saved);
    return { ...empty(), ...data, timing: { ...defaultTiming, ...data.timing } };
  } catch {
    return empty();
  }
}

export const workflow = $state<WorkflowData>(restore());

export function saveWorkflow() {
  try {
    sessionStorage.setItem('workflow', JSON.stringify(workflow));
  } catch {
    // The workflow still works for the current page session if storage is unavailable.
  }
}
