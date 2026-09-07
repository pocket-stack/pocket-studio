import { computed, readonly, ref } from "vue";

/** Factory root password on stock iOS; used whenever nothing else is set. */
export const FACTORY_SSH_PASSWORD = "alpine";
const storageKey = "pocket-studio.default-ssh-password";

function read(): string {
  try {
    return localStorage.getItem(storageKey) ?? "";
  } catch {
    return "";
  }
}

const stored = ref(read());

export function setDefaultSshPassword(value: string): void {
  stored.value = value;
  try {
    if (value) localStorage.setItem(storageKey, value);
    else localStorage.removeItem(storageKey);
  } catch {
    // Preferences are a convenience; a blocked store just means asking again.
  }
}

/** Default root SSH password from Preferences, falling back to the factory one. */
export function useDefaultSshPassword() {
  return {
    stored: readonly(stored),
    effective: computed(() => stored.value || FACTORY_SSH_PASSWORD),
    setDefaultSshPassword,
  };
}
