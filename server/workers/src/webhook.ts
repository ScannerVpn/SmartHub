// ---------------------------------------------------------------------------
// POST /webhook — Lemon Squeezy webhook receiver.
// Verifies the HMAC-SHA256 signature (X-Signature header) against the raw
// body, then records purchase/refund/key events in D1.
// Docs: https://docs.lemonsqueezy.com/help/webhooks
// ---------------------------------------------------------------------------
import type { Env } from './index';
import { hmacSha256Hex, sha256Hex, json } from './lib';

interface WebhookPayload {
  meta?: { event_name?: string; custom_data?: Record<string, unknown> };
  data?: {
    id?: string;
    type?: string;
    attributes?: Record<string, any>;
  };
}

/** Timing-safe comparison for hex digests. */
function timingSafeEqual(a: string, b: string): boolean {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  return diff === 0;
}

export async function handleWebhook(request: Request, env: Env): Promise<Response> {
  if (!env.WEBHOOK_SECRET) {
    console.error('WEBHOOK_SECRET is not configured');
    return json({ error: 'server misconfigured' }, 500);
  }

  const signature = request.headers.get('X-Signature') ?? '';
  const rawBody = await request.text();
  const expected = await hmacSha256Hex(env.WEBHOOK_SECRET, rawBody);

  if (!signature || !timingSafeEqual(signature.toLowerCase(), expected.toLowerCase())) {
    return json({ error: 'invalid signature' }, 401);
  }

  let payload: WebhookPayload;
  try {
    payload = JSON.parse(rawBody);
  } catch {
    return json({ error: 'invalid json' }, 400);
  }

  const eventName = payload.meta?.event_name ?? 'unknown';
  const now = Math.floor(Date.now() / 1000);
  await env.DB.prepare(
    `INSERT INTO events (event_name, payload_json, received_at) VALUES (?1, ?2, ?3)`
  )
    .bind(eventName, rawBody.slice(0, 60_000), now)
    .run();

  // Keep the licenses table roughly in sync for support/refund flows.
  const attrs = payload.data?.attributes;
  const keyValue: string | undefined =
    attrs?.license_key ?? attrs?.key ?? attrs?.license_key_value;
  if (keyValue) {
    const keyHash = await sha256Hex(keyValue);
    const newStatus = mapEventToStatus(eventName, attrs?.status);
    if (newStatus) {
      await env.DB.prepare(`UPDATE licenses SET status = ?1 WHERE license_key_hash = ?2`)
        .bind(newStatus, keyHash)
        .run()
        .catch((err) => console.error('status sync failed', err));
    }
  }

  return json({ ok: true });
}

function mapEventToStatus(eventName: string, lsStatus?: string): string | null {
  if (eventName === 'license_key_created') return 'active';
  if (eventName === 'order_refunded' || eventName === 'subscription_refunded') return 'refunded';
  if (lsStatus === 'expired') return 'expired';
  if (lsStatus === 'disabled') return 'disabled';
  return null;
}
