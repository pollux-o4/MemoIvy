import { joinTitle, splitTitle } from '../../../../shared/model';
import { type DraftCommit, useDraft } from '../../../../shared/draft/useDraft';

/**
 * 메모 텍스트를 제목·본문 두 초안으로 나누는 어댑터. 나누고 합치는 규칙은 model의 splitTitle·joinTitle이 맡는다.
 * 합친 본문은 지금 본문(text)에서 출발하므로 그것을 origin으로 넘겨 본문 외 충돌만 재기준하게 한다.
 * 제목이 빈 채 본문만 쓰면 본문 첫 줄이 제목으로 올라가지 않게 fallbackTitle을 제목으로 채운다.
 */
export function useTitledText(text: string, onSave: (text: string, baseVersion: number | undefined, origin: string) => ReturnType<DraftCommit>, { version, fallbackTitle }: { version?: number; fallbackTitle: string }) {
  const parts = splitTitle(text);
  const title = useDraft(parts.title, (value, base) => onSave(joinTitle({ ...parts, title: value || parts.title }), base, text), { version });
  const body = useDraft(parts.body, (value, base) => onSave(joinTitle({ ...parts, title: parts.title || (value.trim() ? fallbackTitle : ''), body: value }), base, text), { version });
  return { title, body };
}
