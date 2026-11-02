import type { ClipCandidate, Easing } from './types.js';

export interface TimingSettings {
  duration_secs: number;
  viewport_width: number;
  viewport_height: number;
  easing: Easing;
  crop: string;
}

export const defaultTiming: TimingSettings = {
  duration_secs: 60,
  viewport_width: 1080,
  viewport_height: 1920,
  easing: 'linear',
  crop: ''
};

export interface Visibility {
  enter: number;
  exit: number;
  duration: number;
}

// Match the integer scaling and even-pixel rounding in the renderer.
export function clipWidth(clip: ClipCandidate, settings: TimingSettings): number {
  let width =
    clip.height > 0
      ? Math.floor((clip.width * settings.viewport_height) / clip.height)
      : settings.viewport_width;
  width += width % 2;
  const match = /^(\d+):(\d+)$/.exec(settings.crop.trim());
  if (match && +match[1] > 0 && +match[2] > 0) {
    let crop = Math.floor((settings.viewport_height * +match[1]) / +match[2]);
    crop += crop % 2;
    width = Math.min(width, crop);
  }
  return width;
}

export function scrollProgress(time: number, easing: Easing): number {
  return easing === 'linear' ? time : 0.3 * time + 0.7 * time * time * (3 - 2 * time);
}

export function timeAtOffset(fraction: number, easing: Easing): number {
  const target = Math.max(0, Math.min(1, fraction));
  if (easing === 'linear') return target;
  let low = 0,
    high = 1;
  for (let i = 0; i < 50; i++) {
    const mid = (low + high) / 2;
    if (scrollProgress(mid, easing) < target) low = mid;
    else high = mid;
  }
  return (low + high) / 2;
}

// Visibility includes partial entrance/exit and is clipped to the output duration.
export function estimateVisibility(
  clips: ClipCandidate[],
  settings: TimingSettings
): Record<string, Visibility> {
  const widths = clips.map((clip) => clipWidth(clip, settings));
  const distance = widths.reduce((sum, width) => sum + width, 0) - settings.viewport_width;
  if (distance <= 0 || !(settings.duration_secs > 0)) return {};
  let left = 0;
  return Object.fromEntries(
    clips.map((clip, i) => {
      const enter =
        timeAtOffset((left - settings.viewport_width) / distance, settings.easing) *
        settings.duration_secs;
      left += widths[i];
      const exit = timeAtOffset(left / distance, settings.easing) * settings.duration_secs;
      return [clip.id, { enter, exit, duration: exit - enter }];
    })
  );
}

export function excerptRange(
  clip: ClipCandidate,
  start: number,
  visibility?: Visibility
): [number, number] {
  const safeStart = Math.max(0, Math.min(Math.round(start), Math.max(0, clip.duration_ms - 1)));
  return [
    safeStart,
    Math.min(
      clip.duration_ms,
      safeStart + Math.max(1, Math.ceil((visibility?.duration ?? clip.duration_ms / 1000) * 1000))
    )
  ];
}
