export const uid = () => globalThis.crypto?.randomUUID?.().replaceAll('-', '') ?? `${Date.now().toString(36)}${Math.random().toString(36).slice(2)}`;
export const escapeHTML = value => String(value ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
export const clone = value => JSON.parse(JSON.stringify(value));
export function safeURL(value) {
 const url=String(value??'').trim();
 if (/^(https?:\/\/|mailto:|page:[a-zA-Z0-9_-]+$|asset:[a-zA-Z0-9_.-]+$)/i.test(url)) return url;
 return '';
}
