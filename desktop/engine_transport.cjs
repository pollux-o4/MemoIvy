'use strict';

const { spawn } = require('node:child_process');
const { EventEmitter } = require('node:events');
const { StringDecoder } = require('node:string_decoder');

const PROTOCOL_VERSION = 1;
const MAX_LINE_BYTES = 32 * 1024 * 1024;

class EngineTransport extends EventEmitter {
  constructor({ executable, args = [], env = process.env, timeoutMs = 15000, spawnProcess = spawn }) {
    super();
    this.executable = executable;
    this.args = args;
    this.env = env;
    this.timeoutMs = timeoutMs;
    this.spawnProcess = spawnProcess;
    this.child = null;
    this.pending = new Map();
    this.decoder = new StringDecoder('utf8');
    this.buffer = '';
    this.closed = false;
    this.closing = false;
    this.deadError = null;
  }

  start() {
    if (this.child) return;
    if (this.deadError) throw this.deadError;
    if (this.closed) throw new Error('Engine transport is closed');
    const child = this.spawnProcess(this.executable, this.args, {
      env: this.env,
      stdio: ['pipe', 'pipe', 'pipe'],
      windowsHide: true,
    });
    this.child = child;
    child.stdout.on('data', (chunk) => this.#onData(chunk));
    child.stdout.on('error', (error) => this.#fail(error));
    child.stderr.on('data', (chunk) => {
      // Keep protocol stdout clean; expose bounded diagnostics for logs only.
      this.emit('stderr', String(chunk).slice(0, 8192));
    });
    child.on('error', (error) => this.#fail(error));
    child.on('exit', (code, signal) => {
      if (this.closing) return;
      this.#fail(new Error(`Engine exited (${code ?? signal ?? 'unknown'})`));
    });
  }

  request(op, params = {}) {
    if (this.deadError) return Promise.reject(this.deadError);
    if (this.closed) return Promise.reject(new Error('Engine transport is closed'));
    try { this.start(); } catch (error) { return Promise.reject(error); }
    const id = cryptoRandomId();
    const payload = `${JSON.stringify({ version: PROTOCOL_VERSION, id, op, params })}\n`;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`Engine request timed out: ${op}`));
      }, this.timeoutMs);
      this.pending.set(id, { resolve, reject, timer, op });
      this.child.stdin.write(payload, (error) => {
        if (!error) return;
        const entry = this.pending.get(id);
        if (entry) {
          clearTimeout(entry.timer);
          this.pending.delete(id);
          entry.reject(error);
        }
      });
    });
  }

  async shutdown() {
    if (!this.child) return;
    if (!this.closed) {
      try { await this.request('flush', {}); } catch (error) { this.emit('shutdown-error', error); }
      this.closed = true;
      this.closing = true;
      this.child.stdin.end();
    }
  }

  #onData(chunk) {
    this.buffer += this.decoder.write(chunk);
    if (Buffer.byteLength(this.buffer, 'utf8') > MAX_LINE_BYTES) {
      this.#fail(new Error('Engine protocol frame exceeded the size limit'));
      this.child.kill();
      return;
    }
    while (true) {
      const newline = this.buffer.indexOf('\n');
      if (newline < 0) return;
      const line = this.buffer.slice(0, newline);
      this.buffer = this.buffer.slice(newline + 1);
      if (!line.trim()) continue;
      let message;
      try { message = JSON.parse(line); } catch {
        this.#fail(new Error('Engine sent malformed JSON'));
        this.child.kill();
        return;
      }
      if (message?.version !== PROTOCOL_VERSION) {
        this.#fail(new Error('Engine sent an unsupported protocol version'));
        this.child.kill();
        return;
      }
      if (message.event === 'snapshot' || message.event === 'error') {
        this.emit('event', message);
        continue;
      }
      if (typeof message.id !== 'string') continue;
      const entry = this.pending.get(message.id);
      if (!entry) continue;
      clearTimeout(entry.timer);
      this.pending.delete(message.id);
      if (message.error) {
        const error = new Error(message.error.message || 'Engine request failed');
        error.code = message.error.code || 'ENGINE_ERROR';
        error.snapshot = message.error.snapshot;
        entry.reject(error);
      } else if (message.result && typeof message.result === 'object') {
        entry.resolve(message.result);
      } else {
        entry.reject(new Error('Engine response has no result'));
      }
    }
  }

  #fail(error) {
    if (this.deadError || this.closing) return;
    this.deadError = error;
    this.closed = true;
    if (this.child && !this.closing) this.child.kill();
    this.emit('failure', error);
    for (const [id, entry] of this.pending) {
      clearTimeout(entry.timer);
      this.pending.delete(id);
      entry.reject(error);
    }
  }
}

function cryptoRandomId() {
  return require('node:crypto').randomUUID();
}

module.exports = { EngineTransport };
