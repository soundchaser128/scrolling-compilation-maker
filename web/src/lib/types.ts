export interface ClipCandidate {
  id: string;
  content_url: string;
  title: string | null;
  alexandria_url: string;
  file_type: 'video' | 'image';
  mime_type: string;
  width: number;
  height: number;
  duration_ms: number;
  tags: string[];
  performers: string[];
  popularity: number;
}

export type Orientation = 'portrait' | 'landscape' | 'square' | 'any';
export type Codec = 'x264' | 'hevc' | 'av1';
export type Quality = 'low' | 'medium' | 'high' | 'veryhigh';
export type Effort = 'low' | 'medium' | 'high';
export type Easing = 'linear' | 'ease';

export interface ClipSelection {
  id: string;
  content_url: string;
  title: string | null;
  alexandria_url: string;
  file_type: 'video' | 'image';
  mime_type: string;
  width: number;
  height: number;
  start_ms: number;
  end_ms: number;
  performers: string[];
  tags: string[];
}

export interface RenderRequest {
  clips: ClipSelection[];
  viewport_width: number;
  viewport_height: number;
  duration_secs: number;
  codec: Codec;
  quality: Quality;
  effort: Effort;
  gpu: boolean;
  easing: Easing;
  text: 'performers' | 'tags' | null;
  crop: string | null;
  song: string | null;
}

export interface RenderStatus {
  state: 'queued' | 'running' | 'done' | 'failed';
  progress: number;
  output_path: string | null;
  error: string | null;
}
