/** 모듈 경계: app → features → shared, desktop은 화면 코드를 모른다. */
module.exports = {
  forbidden: [
    { name: 'no-circular', severity: 'error', from: {}, to: { circular: true } },
    { name: 'shared-is-leaf', comment: 'shared는 기능·앱을 모른다', severity: 'error', from: { path: '^frontend/shared' }, to: { path: '^frontend/(features|app)' } },
    { name: 'features-are-isolated', comment: '기능끼리는 shared를 통해서만 공유한다', severity: 'error', from: { path: '^frontend/features/([^/]+)/' }, to: { path: '^frontend/features/', pathNot: '^frontend/features/$1/' } },
    { name: 'features-not-app', severity: 'error', from: { path: '^frontend/features' }, to: { path: '^frontend/app' } },
    { name: 'design-is-docs-only', comment: '온보딩·기초 페이지는 앱 코드가 불러오지 않는다', severity: 'error', from: { path: '^frontend/(app|features|shared)' }, to: { path: '^frontend/design' } },
    { name: 'desktop-not-frontend', comment: 'Electron은 화면 코드를 직접 불러오지 않는다', severity: 'error', from: { path: '^desktop' }, to: { path: '^frontend' } },
    { name: 'no-orphans', severity: 'warn', from: { orphan: true, pathNot: ['\\.d\\.ts$', '(^|/)\\.[^/]+', '\\.(stories|test|e2e)\\.tsx?$', '\\.test\\.cjs$', '^desktop/preload\\.cjs$', '^tools/', '^e2e/', 'vite\\.config|vitest\\.config'] }, to: {} },
    { name: 'not-to-unresolvable', severity: 'error', from: {}, to: { couldNotResolve: true } },
  ],
  options: {
    doNotFollow: { path: 'node_modules' },
    exclude: { path: '^(node_modules|target|engine|src|frontend/dist|storybook-static)' },
    tsConfig: { fileName: 'tsconfig.json' },
    parser: 'swc', // typescript@7 API는 dependency-cruiser가 읽지 못한다
    enhancedResolveOptions: { exportsFields: ['exports'], conditionNames: ['import', 'require', 'node', 'default'] },
  },
};
