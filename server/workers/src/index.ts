// ---------------------------------------------------------------------------
// SmartHub licensing worker.
//
//   GET  /health    → liveness probe
//   POST /activate  → activate a license key on a device (proxies Lemon Squeezy)
//   POST /validate  → revalidate (cached 10 min) — the app calls this daily
//   POST /webhook   → Lemon Squeezy webhook receiver (HMAC-verified)
//
// Deploy: see wrangler.toml comments.
// ---------------------------------------------------------------------------
import { handleActivate } from './activate';
import { handleValidate } from './validate';
import { handleWebhook } from './webhook';
import { json } from './lib';

export interface Env {
  DB: D1Database;
  VALIDATION_CACHE: KVNamespace;
  WEBHOOK_SECRET?: string;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const { pathname } = new URL(request.url);

    if (request.method === 'GET' && pathname === '/health') {
      return json({ ok: true, service: 'smarthub-licensing' });
    }
    if (request.method === 'POST' && pathname === '/activate') {
      return handleActivate(request, env);
    }
    if (request.method === 'POST' && pathname === '/validate') {
      return handleValidate(request, env);
    }
    if (request.method === 'POST' && pathname === '/webhook') {
      return handleWebhook(request, env);
    }

    return json({ error: 'not found' }, 404);
  }
} satisfies ExportedHandler<Env>;
