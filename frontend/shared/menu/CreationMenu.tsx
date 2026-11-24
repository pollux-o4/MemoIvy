import { Icon } from '../Icon';
import { MorphMenu } from './MorphMenu';

/** New 버튼: + 가 45° 돌아 닫기가 되는 강조 morph 메뉴. */
export function CreationMenu({ onCreate, disabled = false, projectAllowed = true }: { onCreate: (project: boolean) => void; disabled?: boolean; projectAllowed?: boolean }) {
  return <MorphMenu label="새로 만들기" title="메모 또는 프로젝트 만들기" disabled={disabled} className="creation"
    trigger={() => <><span className="morph-turn"><Icon name="plus"/></span><span className="creation-label">New</span></>}
    choices={[
      { key: 'note', icon: 'note', label: '메모', onSelect: () => onCreate(false) },
      { key: 'project', icon: 'project', label: '프로젝트', disabled: !projectAllowed, title: projectAllowed ? undefined : '이 위치에는 프로젝트를 더 만들 수 없어요', onSelect: () => onCreate(true) },
    ]}/>;
}
