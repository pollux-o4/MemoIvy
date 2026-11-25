import type { DesktopBridge, Snapshot } from './engine_contract';

type Envelope = { __memoivyError?: { message?: string; code?: string; snapshot?: Snapshot } };

/** preload가 평범한 객체로 넘긴 엔진 오류를 code·snapshot을 가진 Error로 되살린다. */
export function engineError(reason: unknown): unknown {
  const data = (reason as Envelope | null)?.__memoivyError;
  if (!data) return reason;
  return Object.assign(new Error(data.message || 'MemoIvy engine request failed'), { code: data.code || 'ENGINE_ERROR', snapshot: data.snapshot });
}

export function withEngineErrors(bridge: DesktopBridge): DesktopBridge {
  const rethrow = (reason: unknown): never => { throw engineError(reason); };
  return {
    ...bridge,
    request: (op, params) => bridge.request(op, params).catch(rethrow),
    importLegacy: () => bridge.importLegacy().catch(rethrow),
  };
}
