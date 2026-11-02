import type { ClipCandidate } from '#lib/types.js';

interface WorkflowData {
  clips: ClipCandidate[] | null;
  selected: string[];
  ranges: Record<string, [number, number]>;
  jobId: string | null;
}

const empty = (): WorkflowData => ({ clips: null, selected: [], ranges: {}, jobId: null });

function restore(): WorkflowData {
  try {
    const saved = sessionStorage.getItem('workflow');
    return saved ? { ...empty(), ...JSON.parse(saved) } : empty();
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
