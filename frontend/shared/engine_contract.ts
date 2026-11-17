/** Desktop/renderer/engine contract v1. Rust mirrors these JSON shapes. */
export interface CanvasNode { position: [number, number] | null; next: string[]; done: boolean; project?: boolean; parent?: string | null; pinned?: boolean }
export interface Note { id: string; text: string; color: string; modified: number; canvas: CanvasNode }
export interface Asset { id: string; name: string; mime: string; bytes: number; usedBy: string[]; added?: number; lastNote?: string | null; unusedSince?: number | null }
export interface Snapshot { notes: Note[]; revision: number; savedRevision: number; assets: Asset[]; noteRevisions?: Record<string, number>; preferences?: Record<string, unknown> }
export interface EngineError { code: string; message: string; snapshot?: Snapshot }
export interface EngineResult { snapshot?: Snapshot; note?: Note; asset?: Asset; data?: string; mime?: string }
export interface EngineRequest { version: 1; id: string; op: string; params?: Record<string, unknown> }
export interface EngineResponse { version: 1; id: string; result?: EngineResult; error?: EngineError }
export interface EngineEvent { version: 1; event: 'snapshot' | 'error'; snapshot?: Snapshot; error?: EngineError }
export interface DesktopBridge {
  request(op: string, params?: Record<string, unknown>): Promise<EngineResult>;
  subscribe(listener: (event: EngineEvent) => void): () => void;
  openNote(id: string): Promise<void>;
  openWindow(kind: 'library' | 'canvas' | 'files', id?: string): Promise<void>;
  openExternal(url: string): Promise<void>;
  importLegacy(): Promise<EngineResult | null>;
  windowAction(action: 'minimize' | 'maximize' | 'close'): void;
  onFlush(listener: () => Promise<void>): () => void;
}
declare global { interface Window { memoivy?: DesktopBridge } }
