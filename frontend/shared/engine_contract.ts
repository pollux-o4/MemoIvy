/** Desktop/renderer/engine contract v1. Rust mirrors these JSON shapes. */
export interface CanvasNode { position: [number, number] | null; next: string[]; done: boolean; project?: boolean; parent?: string | null; pinned?: boolean }
export interface Note { id: string; text: string; color: string; modified: number; canvas: CanvasNode }
export interface Asset { id: string; name: string; mime: string; bytes: number; usedBy: string[]; added?: number; lastNote?: string | null; unusedSince?: number | null }
export interface Snapshot { notes: Note[]; revision: number; savedRevision: number; assets: Asset[]; noteRevisions?: Record<string, number>; preferences?: Record<string, unknown> }
/** @public Rust 엔진과 맞춰 보는 프로토콜 형태 */
export interface EngineError { code: string; message: string; snapshot?: Snapshot }
export interface EngineResult { snapshot?: Snapshot; note?: Note; asset?: Asset; data?: string; mime?: string }
/**
 * 충돌 규칙: params.baseRevision은 사용자가 작업을 시작할 때 본 전역 snapshot.revision이다.
 * 엔진은 대상의 noteRevisions[id]가 그 뒤에 바뀐 경우에만 CONFLICT를 낸다(다른 메모·설정 변경은 막지 않는다).
 * 편집 초안은 처음 더러워진 시점, 드래그는 pointerdown 시점의 값을 유지하며 새 snapshot만으로 올리지 않는다.
 */
/** @public Rust 엔진과 맞춰 보는 프로토콜 형태 */
export interface EngineRequest { version: 1; id: string; op: string; params?: Record<string, unknown> }
/** @public Rust 엔진과 맞춰 보는 프로토콜 형태 */
export interface EngineResponse { version: 1; id: string; result?: EngineResult; error?: EngineError }
export interface EngineEvent { version: 1; event: 'snapshot' | 'error'; snapshot?: Snapshot; error?: EngineError }
/** 창 관리자가 정한 창 상태. 메인 프로세스의 maximize·fullscreen 이벤트가 단일 출처다. */
export type WindowState = { maximized: boolean };

export interface DesktopBridge {
  request(op: string, params?: Record<string, unknown>): Promise<EngineResult>;
  subscribe(listener: (event: EngineEvent) => void): () => void;
  openNote(id: string): Promise<void>;
  openWindow(kind: 'library' | 'canvas' | 'files' | 'settings' | 'help', id?: string): Promise<void>;
  openExternal(url: string): Promise<void>;
  importLegacy(): Promise<EngineResult | null>;
  windowAction(action: 'minimize' | 'maximize' | 'close'): void;
  onFlush(listener: () => Promise<void>): () => void;
  /** 구독 즉시 현재 상태를 한 번 받고, 바뀔 때마다 다시 받는다. */
  onWindowState(listener: (state: WindowState) => void): () => void;
}
declare global { interface Window { memoivy?: DesktopBridge } }
