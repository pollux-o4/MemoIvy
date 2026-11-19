'use strict';

const MAX_URL_LENGTH = 4096;

function isSafeExternal(url) {
  if (typeof url !== 'string' || url.length > MAX_URL_LENGTH) return false;
  try {
    const parsed = new URL(url);
    return ['https:', 'http:', 'mailto:'].includes(parsed.protocol) && !parsed.username && !parsed.password;
  } catch { return false; }
}

function openExternal(url, open) {
  return Promise.resolve().then(() => {
    if (!isSafeExternal(url)) throw new TypeError('Only HTTP, HTTPS, and mailto links are allowed');
    return open(url);
  });
}

module.exports = { isSafeExternal, openExternal };
