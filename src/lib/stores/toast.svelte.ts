// ---------------------------------------------------------------------------
// Tiny toast store for transient feedback ("Copied!", "Profile applied"…).
// ---------------------------------------------------------------------------
export interface Toast {
  id: number;
  message: string;
  kind: 'success' | 'error' | 'info';
}

function createToastStore() {
  let toasts = $state<Toast[]>([]);
  let seq = 0;

  function push(message: string, kind: Toast['kind'] = 'info', ms = 2200) {
    const id = ++seq;
    toasts = [...toasts, { id, message, kind }];
    setTimeout(() => {
      toasts = toasts.filter((t) => t.id !== id);
    }, ms);
  }

  return {
    get list() {
      return toasts;
    },
    success: (m: string) => push(m, 'success'),
    error: (m: string) => push(m, 'error', 3600),
    info: (m: string) => push(m, 'info')
  };
}

export const toast = createToastStore();
