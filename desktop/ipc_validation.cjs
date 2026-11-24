'use strict';

const OPERATIONS = new Set([
  'snapshot', 'create', 'update', 'delete', 'canvas', 'restore', 'flush',
  'importAsset', 'readAsset', 'deleteAsset', 'preference',
]);
const ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const ASSET_ID = /^[0-9a-f]{64}$/;
const COLORS = new Set(['yellow', 'pink', 'blue', 'green', 'purple']);
// 메모 창은 openNote 경로로만 연다.
const WINDOW_KINDS = new Set(Object.keys(require('./window_kinds.json')).filter((kind) => kind !== 'note'));
// 캔버스 좌표 범위의 단일 출처(렌더러·엔진 테스트도 같은 파일을 읽는다).
const { limit: CANVAS_LIMIT } = require('./canvas_bounds.json');
const MAX_TEXT = 28 * 1024 * 1024;
const MAX_ASSET_BYTES = 20 * 1024 * 1024;

function validateRequest(op, params) {
  if (typeof op !== 'string' || !OPERATIONS.has(op)) throw bad('Unsupported operation');
  if (!plainObject(params)) throw bad('Parameters must be an object');
  const allowed = {
    snapshot: [], flush: [],
    create: ['baseRevision', 'text', 'color', 'parent', 'project', 'position'],
    update: ['baseRevision', 'id', 'text', 'color', 'canvas'],
    delete: ['baseRevision', 'id', 'ids'],
    canvas: ['baseRevision', 'changes'],
    restore: ['baseRevision', 'notes', 'changes'],
    importAsset: ['baseRevision', 'name', 'data', 'mime'],
    readAsset: ['id'], deleteAsset: ['baseRevision', 'id'], preference: ['baseRevision', 'key', 'value'],
  }[op];
  exactKeys(params, allowed);
  if (['create', 'update', 'delete', 'canvas', 'restore', 'importAsset', 'deleteAsset', 'preference'].includes(op)) revision(params.baseRevision);
  if (op === 'create') {
    optionalText(params.text, MAX_TEXT, 'text');
    optionalColor(params.color);
    if (params.parent !== undefined && params.parent !== null && !isId(params.parent)) throw bad('Invalid parent id');
    if (params.project !== undefined && typeof params.project !== 'boolean') throw bad('Invalid project flag');
    if (params.position !== undefined && params.position !== null && !position(params.position)) throw bad('Invalid position');
  } else if (op === 'update') {
    id(params.id);
    optionalText(params.text, MAX_TEXT, 'text');
    optionalColor(params.color);
    if (params.canvas !== undefined) canvas(params.canvas);
  } else if (op === 'delete') {
    // 한 건(id) 또는 여러 건(ids) 중 하나만 받는다.
    if ((params.id === undefined) === (params.ids === undefined)) throw bad('Give id or ids');
    if (params.ids === undefined) id(params.id);
    else if (!Array.isArray(params.ids) || !params.ids.length || params.ids.length > 5000) throw bad('Invalid note ids');
    else params.ids.forEach(id);
  } else if (op === 'deleteAsset') {
    assetId(params.id);
  } else if (op === 'canvas') canvasChanges(params.changes);
  else if (op === 'restore') {
    if (!Array.isArray(params.notes) || !params.notes.length || params.notes.length > 5000) throw bad('Invalid restored notes');
    for (const note of params.notes) {
      if (!plainObject(note)) throw bad('Invalid restored note');
      exactKeys(note, ['id', 'text', 'color', 'modified', 'canvas']);
      id(note.id);
      if (typeof note.text !== 'string') throw bad('Invalid text');
      optionalText(note.text, MAX_TEXT, 'text');
      if (typeof note.color !== 'string') throw bad('Invalid color');
      optionalColor(note.color);
      if (!Number.isSafeInteger(note.modified) || note.modified < 0) throw bad('Invalid modified time');
      canvas(note.canvas);
    }
    canvasChanges(params.changes);
  } else if (op === 'readAsset') assetId(params.id);
  else if (op === 'preference') validatePreference(params.key, params.value);
  else if (op === 'importAsset') {
    if (typeof params.name !== 'string' || !params.name.trim() || params.name.length > 255 || /[\\/\0]/.test(params.name)) throw bad('Invalid asset name');
    if (typeof params.data !== 'string' || !params.data.length || params.data.length > Math.ceil(MAX_ASSET_BYTES * 4 / 3) || params.data.length % 4 !== 0 || !/^[A-Za-z0-9+/]*={0,2}$/.test(params.data)) throw bad('Invalid asset data');
    const bytes = Buffer.from(params.data, 'base64');
    if (!bytes.length || bytes.length > MAX_ASSET_BYTES || bytes.toString('base64') !== params.data) throw bad('Invalid asset data');
    if (params.mime !== undefined && (typeof params.mime !== 'string' || params.mime.length > 127 || !/^[\w.+-]+\/[\w.+-]+$/.test(params.mime))) throw bad('Invalid MIME type');
  }
  return params;
}

function canvasChanges(changes) {
  if (!Array.isArray(changes) || changes.length > 5000) throw bad('Invalid canvas changes');
  for (const change of changes) {
    if (!plainObject(change)) throw bad('Invalid canvas change');
    exactKeys(change, ['id', 'canvas']);
    id(change.id);
    canvas(change.canvas);
  }
}

function validateWindowTarget(kind, id) {
  if (!WINDOW_KINDS.has(kind) || (id !== undefined && !isId(id))) throw new TypeError('Invalid window target');
  return { kind, id };
}

function validatePreference(key, value) {
  if (key === 'autoPrune') {
    if (typeof value !== 'boolean') throw bad('Invalid autoPrune preference');
    return;
  }
  if (key === 'expandedProjects' || key === 'expandedNotes') {
    if (!Array.isArray(value) || value.length > 10000 || !value.every(isId)) throw bad(`Invalid ${key} preference`);
    return;
  }
  if (key === 'navigation') {
    if (!plainObject(value) || Object.keys(value).length > 10000) throw bad('Invalid navigation preference');
    for (const [noteId, stats] of Object.entries(value)) {
      if (!isId(noteId) || !plainObject(stats)) throw bad('Invalid navigation entry');
      exactKeys(stats, ['count', 'lastVisited']);
      if (!Number.isSafeInteger(stats.count) || stats.count < 0 || !Number.isFinite(stats.lastVisited) || stats.lastVisited < 0) throw bad('Invalid navigation statistics');
    }
    return;
  }
  if (key === 'appearance') {
    if (!plainObject(value)) throw bad('Invalid appearance preference');
    exactKeys(value, ['theme', 'motion']);
    if (!['system', 'light', 'dark'].includes(value.theme) || !['system', 'reduce', 'full'].includes(value.motion)) throw bad('Invalid appearance preference');
    return;
  }
  if (key === 'canvasLinkStyle') {
    if (!['curve', 'straight', 'orthogonal'].includes(value)) throw bad('Invalid canvasLinkStyle preference');
    return;
  }
  throw bad('Unsupported preference key');
}

function canvas(value) {
  if (!plainObject(value)) throw bad('Canvas must be an object');
  exactKeys(value, ['position', 'next', 'done', 'project', 'parent', 'pinned']);
  if (value.position !== undefined && value.position !== null && !position(value.position)) throw bad('Invalid position');
  if (value.next !== undefined && (!Array.isArray(value.next) || value.next.length > 10000 || !value.next.every(isId))) throw bad('Invalid canvas links');
  for (const key of ['done', 'project', 'pinned']) if (value[key] !== undefined && typeof value[key] !== 'boolean') throw bad(`Invalid ${key} flag`);
  if (value.parent !== undefined && value.parent !== null && !isId(value.parent)) throw bad('Invalid parent id');
}

function position(value) { return Array.isArray(value) && value.length === 2 && value.every((n) => Number.isFinite(n) && n >= -CANVAS_LIMIT && n <= CANVAS_LIMIT); }
function optionalText(value, limit, name) { if (value !== undefined && (typeof value !== 'string' || Buffer.byteLength(value, 'utf8') > limit)) throw bad(`Invalid ${name}`); }
function optionalColor(value) { if (value !== undefined && (typeof value !== 'string' || !COLORS.has(value))) throw bad('Invalid color'); }
function revision(value) { if (!Number.isSafeInteger(value) || value < 0) throw bad('Invalid base revision'); }
function isId(value) { return typeof value === 'string' && ID.test(value); }
function id(value) { if (!isId(value)) throw bad('Invalid note id'); }
function assetId(value) { if (typeof value !== 'string' || !ASSET_ID.test(value)) throw bad('Invalid asset id'); }
function plainObject(value) { return value !== null && typeof value === 'object' && !Array.isArray(value) && (Object.getPrototypeOf(value) === Object.prototype || Object.getPrototypeOf(value) === null); }
function exactKeys(value, allowed) { if (Object.keys(value).some((key) => !allowed.includes(key))) throw bad('Unexpected parameter'); }
function bad(message) { const error = new TypeError(message); error.code = 'INVALID_REQUEST'; return error; }

module.exports = { validateRequest, validateWindowTarget, isId };
