import type { Alignment } from './align';

type AlignIcon = 'alignLeft' | 'alignCenterX' | 'alignRight' | 'alignTop' | 'alignCenterY' | 'alignBottom' | 'distributeX' | 'distributeY' | 'tidy' | 'flow';
/** Alt(+Shift)+글자 하나. 표시 문자열과 키 판정이 모두 여기서 나온다. */
interface Chord { key: string; shift?: true }
interface AlignAction { kind: Alignment; label: string; chord: Chord; icon: AlignIcon; minCount: number }
export interface AlignGroup { id: 'align' | 'arrange'; label: string; icon: AlignIcon; rows: readonly (readonly AlignAction[])[] }

const action = (kind: Alignment, label: string, key: string, icon: AlignIcon, minCount = 2, shift?: true): AlignAction => ({ kind, label, chord: { key, shift }, icon, minCount });

/** 정렬 동작의 단일 출처: 정렬 막대 묶음·단축키 판정·툴팁. 표 원문은 CANVAS-003. 행은 같은 축끼리 모은다. */
export const ALIGN_GROUPS: readonly AlignGroup[] = [
  { id: 'align', label: '맞춤', icon: 'alignLeft', rows: [
    [action('left', '왼쪽 맞춤', 'a', 'alignLeft'), action('center', '가로 가운데 맞춤', 'h', 'alignCenterX'), action('right', '오른쪽 맞춤', 'd', 'alignRight')],
    [action('top', '위쪽 맞춤', 'w', 'alignTop'), action('middle', '세로 가운데 맞춤', 'v', 'alignCenterY'), action('bottom', '아래쪽 맞춤', 's', 'alignBottom')],
  ] },
  { id: 'arrange', label: '배치', icon: 'tidy', rows: [
    [action('distribute-x', '가로 간격 같게', 'h', 'distributeX', 3, true), action('distribute-y', '세로 간격 같게', 'v', 'distributeY', 3, true)],
    [action('grid', '격자로 정리', 't', 'tidy'), action('flow', '연결 순서로 배치', 'f', 'flow'), action('mindmap', '마인드맵으로 배치', 'm', 'flow', 1)],
  ] },
];

const ACTIONS = ALIGN_GROUPS.flatMap((group) => group.rows.flat());

export const chordLabel = ({ key, shift }: Chord) => `Alt+${shift ? 'Shift+' : ''}${key.toUpperCase()}`;

/** Alt(+Shift)+글자 입력에 맞는 정렬. Ctrl·Meta가 함께 눌리면 정렬이 아니다. */
export function alignmentForKey(event: { key: string; altKey: boolean; shiftKey: boolean; ctrlKey: boolean; metaKey: boolean }): Alignment | undefined {
  if (!event.altKey || event.ctrlKey || event.metaKey) return undefined;
  const key = event.key.toLowerCase();
  return ACTIONS.find(({ chord }) => chord.key === key && Boolean(chord.shift) === event.shiftKey)?.kind;
}
