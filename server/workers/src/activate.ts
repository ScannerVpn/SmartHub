// ---------------------------------------------------------------------------
// POST /activate — { license_key, instance_name }
// Proxies LS activation and records the device in D1 (idempotent).
// ---------------------------------------------------------------------------
import type { Env } from './index';
import { lsActivate } from './lemonsqueezy';
import { sha256Hex, json, badRequest } from './lib';

export async function handleActivate(request: Request, env: Env): Promise<Response> {
  let body: { license_key?: string; instance_name?: string };
  try {
    body = await request.json();
  } catch {
    return badRequest('Expected JSON body');
  }

  const key = body.license_key?.trim();
  if (!key || key.length < 12) return badRequest('Missing or malformed license_key');
  const instanceName = (body.instance_name ?? 'unknown-device').slice(0, 80);

  let ls;
  try {
    ls = await lsActivate(key, instanceName);
  } catch (err) {
    return json({ activated: false, error: `Upstream error: ${String(err).slice(0, 200)}` }, 502);
  }

  if (ls.activated && ls.instance) {
    const now = Math.floor(Date.now() / 1000);
    const keyHash = await sha256Hex(key);
    // Upsert is idempotent: re-activating the same device just refreshes data.
    await env.DB.prepare(
      `INSERT INTO licenses
         (license_key_hash, license_key_id, instance_id, status, customer_email,
          product_name, variant_name, order_id, activation_count, activated_at, created_at)
       VALUES (?1, ?2, ?3, 'active', ?4, ?5, ?6, ?7, 1, ?8, ?8)
       ON CONFLICT(license_key_hash, instance_id) DO UPDATE SET
         status = 'active',
         license_key_id = excluded.license_key_id,
         customer_email = excluded.customer_email,
         product_name = excluded.product_name,
         variant_name = excluded.variant_name,
         order_id = excluded.order_id,
         activation_count = licenses.activation_count + 1`
    )
      .bind(
        keyHash,
        ls.license_key?.id ?? null,
        ls.instance.id,
        ls.license_key?.customer_email ?? null,
        ls.license_key?.product_name ?? null,
        ls.license_key?.variant_name ?? null,
        ls.license_key?.order_id ?? null,
        now
      )
      .run()
      .catch((err) => console.error('D1 activation upsert failed', err));
  }

  // Respond in exactly the shape the desktop app expects.
  return json({
    activated: ls.activated,
    error: ls.error ?? null,
    instance: ls.instance ? { id: ls.instance.id } : undefined,
    license_key: ls.license_key ? { id: ls.license_key.id } : undefined
  });
}
