import type { ReactNode, SVGProps } from 'react';
import add from './assets/icons/actions/note/add.svg?raw';
import more from './assets/icons/actions/more.svg?raw';
import palette from './assets/icons/actions/palette.svg?raw';
import copy from './assets/icons/actions/media/copy.svg?raw';
import expand from './assets/icons/actions/media/expand.svg?raw';
import files from './assets/icons/actions/media/files.svg?raw';
import keyboard from './assets/icons/actions/keyboard.svg?raw';
import note from './assets/icons/actions/note/note.svg?raw';
import source from './assets/icons/actions/note/source.svg?raw';
import trash from './assets/icons/actions/note/trash.svg?raw';
import alignFlow from './assets/icons/align/flow.svg?raw';
import alignTidy from './assets/icons/align/tidy.svg?raw';
import alignLeft from './assets/icons/align/position/left.svg?raw';
import alignCenterX from './assets/icons/align/spacing/center-x.svg?raw';
import alignRight from './assets/icons/align/position/right.svg?raw';
import alignTop from './assets/icons/align/position/top.svg?raw';
import alignCenterY from './assets/icons/align/spacing/center-y.svg?raw';
import alignBottom from './assets/icons/align/position/bottom.svg?raw';
import distributeX from './assets/icons/align/spacing/distribute-x.svg?raw';
import distributeY from './assets/icons/align/spacing/distribute-y.svg?raw';
import canvasCheck from './assets/icons/canvas/flow/check.svg?raw';
import canvasLink from './assets/icons/canvas/flow/link.svg?raw';
import signal from './assets/icons/canvas/flow/signal.svg?raw';
import help from './assets/icons/canvas/help.svg?raw';
import undo from './assets/icons/canvas/history/undo.svg?raw';
import redo from './assets/icons/canvas/history/redo.svg?raw';
import chevronRight from './assets/icons/canvas/navigation/chevron.svg?raw';
import home from './assets/icons/canvas/navigation/home.svg?raw';
import compass from './assets/icons/canvas/navigation/compass.svg?raw';
import pin from './assets/icons/canvas/navigation/pin.svg?raw';
import open from './assets/icons/canvas/open.svg?raw';
import enter from './assets/icons/canvas/project/enter.svg?raw';
import project from './assets/icons/canvas/project/project.svg?raw';
import up from './assets/icons/canvas/project/up.svg?raw';
import fit from './assets/icons/canvas/zoom/fit.svg?raw';
import zoomIn from './assets/icons/canvas/zoom/zoom-in.svg?raw';
import zoomOut from './assets/icons/canvas/zoom/zoom-out.svg?raw';
import close from './assets/icons/window/close.svg?raw';
import maximize from './assets/icons/window/maximize.svg?raw';
import minimize from './assets/icons/window/minimize.svg?raw';
import restore from './assets/icons/window/restore.svg?raw';

const fallbackPaths = {
  search: <><circle cx="11" cy="11" r="7"/><path d="m20 20-4-4"/></>,
  grid: <><rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/></>,
  clock: <><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></>,
  chevron: <path d="m7 10 5 5 5-5"/>,
  settings: <><path d="M4 7h10M18 7h2M4 17h2M10 17h10"/><circle cx="16" cy="7" r="2"/><circle cx="8" cy="17" r="2"/></>,
} satisfies Record<string, ReactNode>;

const svgFiles = {
  add, plus: add, more, palette, copy, expand, files, file: files, image: files,
  keyboard, note, source, trash, alignFlow, flow: alignFlow, canvas: alignFlow,
  alignTidy, tidy: alignTidy, align: alignTidy, alignLeft, alignCenterX, alignRight, alignTop,
  alignCenterY, alignBottom, distributeX, distributeY, canvasCheck, check: canvasCheck,
  canvasLink, link: canvasLink, signal, help, undo, redo, chevronRight, home, compass, pin,
  open, enter, arrow: enter, project, up, fit, zoomIn, zoomOut, close, maximize,
  minimize, restore,
} as const;

type SvgName = keyof typeof svgFiles;
type FallbackName = keyof typeof fallbackPaths;
export type IconName = SvgName | FallbackName;
type IconProps = Omit<SVGProps<SVGSVGElement>, 'children' | 'dangerouslySetInnerHTML'> & {
  name: IconName;
  size?: number;
};

const svgBodies: Record<SvgName, string> = Object.fromEntries(
  Object.entries(svgFiles).map(([name, svg]) => [name, innerSvg(svg)]),
) as Record<SvgName, string>;

function innerSvg(svg: string): string {
  return svg
    .replace(/^\s*<svg\b[^>]*>/, '')
    .replace(/<\/svg>\s*$/, '');
}

export function Icon({ name, size = 20, ...props }: IconProps) {
  const body = name in svgBodies ? svgBodies[name as SvgName] : undefined;
  if (body !== undefined) {
    return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" {...props} dangerouslySetInnerHTML={{ __html: body }}/>;
  }
  const fallback = fallbackPaths[name as FallbackName];
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" {...props}>{fallback}</svg>;
}
