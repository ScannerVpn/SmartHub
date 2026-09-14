// ---------------------------------------------------------------------------
// Minimal typed client for the Lemon Squeezy *license* endpoints.
// These endpoints are unauthenticated by design (the license key IS the
// credential), so no API key is needed server-side either.
// Docs: https://docs.lemonsqueezy.com/api/licenses
// ---------------------------------------------------------------------------

const LS_BASE = 'https://api.lemonsqueezy.com/v1/licenses';

export interface LsActivateResponse {
  activated: boolean;
  error?: string | null;
  license_key?: {
    id: number;
    status: string;
    key: string;
    activation_limit: number | null;
    activation_usage: number;
    customer_email?: string;
    product_name?: string;
    variant_name?: string;
    order_id?: number;
  };
  instance?: { id: string; name: string; created_at: string };
  meta?: Record<string, unknown>;
}

export interface LsValidateResponse {
  valid: boolean;
  error?: string | null;
  license_key?: LsActivateResponse['license_key'];
  instance?: { id: string; name: string; created_at: string };
}

async function post<T>(path: string, body: Record<string, string | undefined>): Promise<T> {
  // LS accepts application/x-www-form-urlencoded for these endpoints.
  const form = new URLSearchParams();
  for (const [k, v] of Object.entries(body)) {
    if (v !== undefined) form.set(k, v);
  }
  const resp = await fetch(`${LS_BASE}${path}`, {
    method: 'POST',
    headers: {
      Accept: 'application/json',
      'Content-Type': 'application/x-www-form-urlencoded'
    },
    body: form.toString()
  });
  if (!resp.ok) {
    // LS returns JSON error bodies on 4xx; surface them cleanly.
    const text = await resp.text().catch(() => '');
    throw new Error(`Lemon Squeezy ${resp.status}: ${text.slice(0, 300)}`);
  }
  return (await resp.json()) as T;
}

export const lsActivate = (licenseKey: string, instanceName: string) =>
  post<LsActivateResponse>('/activate', {
    license_key: licenseKey,
    instance_name: instanceName
  });

export const lsValidate = (licenseKey: string, instanceId?: string) =>
  post<LsValidateResponse>('/validate', {
    license_key: licenseKey,
    instance_id: instanceId
  });
