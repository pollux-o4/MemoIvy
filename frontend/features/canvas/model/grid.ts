export function gridStyle(camera: { x: number; y: number; zoom: number }): Record<'--grid-size' | '--grid-gap' | '--grid-x' | '--grid-y' | '--grid-minor-opacity', string> {
  const gridStep = 24 * 2 ** Math.max(0, Math.ceil(Math.log2(16 / (24 * camera.zoom))));
  const gridGap = gridStep * camera.zoom;
  const gridMajor = gridGap * 2;
  const gridX = ((-camera.x * camera.zoom) % gridMajor + gridMajor) % gridMajor;
  const gridY = ((-camera.y * camera.zoom) % gridMajor + gridMajor) % gridMajor;
  const minorDotOpacity = Math.max(0, Math.min(1, (gridGap - 16) / 16));

  return {
    '--grid-size': `${gridMajor}px`,
    '--grid-gap': `${gridGap}px`,
    '--grid-x': `${gridX}px`,
    '--grid-y': `${gridY}px`,
    '--grid-minor-opacity': `${minorDotOpacity * 100}%`,
  };
}
