import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type JobKind = "progress" | "log" | "title" | "done" | "error" | "cancelled";

export interface JobEvent {
  id: string;
  kind: JobKind;
  percent?: number;
  speed?: string;
  eta?: string;
  elapsed?: number;
  message?: string;
  output?: string;
}

export interface SubsOptions {
  langs: string[];
  only: boolean;
  format: "srt" | "vtt";
  embed: boolean;
}

export interface DownloadRequest {
  id: string;
  url: string;
  mode: "video" | "audio";
  maxHeight?: number;
  audioFormat?: string;
  outDir: string;
  subs?: SubsOptions;
  playlist: boolean;
  cookiesBrowser?: string;
}

export type Quality = "high" | "medium" | "low";

export interface ConvertRequest {
  id: string;
  input: string;
  preset: string;
  outDir: string;
  rotate?: string;
  start?: string;
  end?: string;
  frameEvery?: number;
  height?: number;
  quality?: Quality;
  audioBitrate?: number;
  gifWidth?: number;
  gifFps?: number;
}

export interface PhotoRequest {
  id: string;
  input: string;
  outDir: string;
  format: "same" | "jpg" | "png" | "webp";
  width?: number;
  percent?: number;
  quality: number;
}

export interface Clip {
  path: string;
  start: number;
  end?: number;
}

export interface AudioEditRequest {
  id: string;
  input: string;
  outDir: string;
  format: string;
  bitrate?: number;
  duration: number;
  mode: "all" | "keep" | "delete";
  selStart: number;
  selEnd: number;
  fadeIn: number;
  fadeOut: number;
  volumeDb: number;
  normalize: boolean;
}

export interface RecordRequest {
  id: string;
  outDir: string;
  name: string;
  fps: number;
  audioDevice?: string;
  cursor: boolean;
}

export interface HeightInfo {
  height: number;
  fps?: number;
  size?: number;
}

export interface SubLang {
  code: string;
  name: string;
  auto: boolean;
}

export interface Probe {
  kind: "video" | "playlist";
  title: string;
  uploader?: string;
  thumbnail?: string;
  duration?: number;
  count?: number;
  entries: string[];
  heights: HeightInfo[];
  audioSize?: number;
  subs: SubLang[];
}

export interface MediaInfo {
  duration?: number;
  width?: number;
  height?: number;
  hasVideo: boolean;
  hasAudio: boolean;
}

export interface FileInfo {
  size: number;
  width?: number;
  height?: number;
  duration?: number;
}

export const api = {
  fileInfo: (path: string) => invoke<FileInfo>("file_info", { path }),
  probe: (url: string, playlist: boolean, cookiesBrowser?: string) =>
    invoke<Probe>("probe", { url, playlist, cookiesBrowser }),
  download: (req: DownloadRequest) => invoke<void>("download", { req }),
  convert: (req: ConvertRequest) => invoke<void>("convert", { req }),
  photo: (req: PhotoRequest) => invoke<void>("photo", { req }),
  mediaInfo: (path: string) => invoke<MediaInfo>("media_info", { path }),
  waveform: (path: string, points: number) =>
    invoke<{ peaks: number[]; duration: number }>("waveform", { path, points }),
  joinClips: (req: { id: string; clips: Clip[]; outDir: string; height?: number; quality?: Quality }) =>
    invoke<void>("join_clips", { req }),
  audioEdit: (req: AudioEditRequest) => invoke<void>("audio_edit", { req }),
  listAudioDevices: () => invoke<string[]>("list_audio_devices"),
  recordScreen: (req: RecordRequest) => invoke<void>("record_screen", { req }),
  stopRecording: (id: string) => invoke<void>("stop_recording", { id }),
  cancel: (id: string) => invoke<void>("cancel_job", { id }),
  versions: () => invoke<{ ytDlp: string; ffmpeg: string }>("versions"),
  updateYtDlp: () => invoke<string>("update_ytdlp"),
  onJob: (cb: (e: JobEvent) => void): Promise<UnlistenFn> =>
    listen<JobEvent>("job", (e) => cb(e.payload)),
};
