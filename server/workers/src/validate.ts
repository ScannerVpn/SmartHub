// ---------------------------------------------------------------------------
// POST /validate — { license_key, instance_id? }
// Validates via LS with a 10-minute KV cache (the app calls this daily, so
// the cache mostly matters when many devices share a network blip/storm).
// ---------------------------------------------------------------------------
import type { Env } from './index';
import { lsValidate } from './lemonsqueezy';
import { sha256Hex, json, badRequest } from './lib';

const CACHE_TTL_SECONDS = 600;

export async function handleValidate(request: Request, env: Env): Promise<Response> {
  let body: { license_key?: string; instance_id?: string };
  try {
    body = await request.json();
  } catch {
    return badRequest('Expected JSON body');
  }

  const key = body.license_key?.trim();
  if (!key) return badRequest('Missing license_key');
  const instanceId = body.instance_id?.trim() || undefined;

  const cacheKey = `validate:${await sha256Hex(`${key}:${instanceId ?? ''}`)}`;

  const cached = await env.VALIDATION_CACHE.get(cacheKey);
  if (cached !== null) {
    return json({ valid: cached === '1', cache: 'hit' });
  }

  let ls;
  try {
    ls = await lsValidate(key, instanceId);
  } catch (err) {
    // Upstream unreachable → let the client's 7-day grace handle it.
    return json({ valid: false, upstream_error: String(err).slice(0, 200) }, 502);
  }

  await env.VALIDATION_CACHE.put(cacheKey, ls.valid ? '1' : '0', {
    expirationTtl: CACHE_TTL_SECONDS
  });

  // Bookkeeping: last validation time (non-blocking is fine here — validation
  // volume is low, and correctness of this timestamp is not critical).
  if (instanceId) {
    const now = Math.floor(Date.now() / 1000);
    const keyHash = await sha256Hex(key);
    await env.DB.prepare(
      `UPDATE licenses SET last_validated_at = ?1
       WHERE license_key_hash = ?2 AND instance_id = ?3`
    )
      .bind(now, keyHash, instanceId)
      .run()
      .catch((err) => console.error('D1 validation stamp failed', err));
  }

  return json({ valid: ls.valid });
}
