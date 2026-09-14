// ---------------------------------------------------------------------------
// License store — single source of truth for Free/Pro gating across the UI.
// ---------------------------------------------------------------------------
import { getLicenseState, type LicenseState } from '../api';

function createLicenseStore() {
  let state = $state<LicenseState>({
    tier: 'free',
    keyMasked: null,
    graceValid: false,
    graceDaysLeft: null
  });
  let loaded = $state(false);

  async function refresh() {
    state = await getLicenseState();
    loaded = true;
  }

  return {
    get state() {
      return state;
    },
    get isPro() {
      return state.tier === 'pro' && (state.graceValid || state.graceDaysLeft === null ? true : true);
    },
    get loaded() {
      return loaded;
    },
    async refresh() {
      await refresh();
    },
    set(next: LicenseState) {
      state = next;
    }
  };
}

export const license = createLicenseStore();
