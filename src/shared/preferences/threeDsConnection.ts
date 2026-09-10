import { computed, readonly, ref, watch } from "vue";

/**
 * Where the user expects their console to be. Studio asks this address on
 * every device scan and uses the ftpd account for wireless preparation, so a
 * paired 3DS reconnects without the user opening the connect flow again.
 */
export interface ThreeDsConnection {
  address: string;
  port: string;
  username: string;
  password: string;
}
export const DEFAULT_FTP_PORT = "5000";
export const DEFAULT_FTP_USERNAME = "anonymous";
const storageKey = "pocket-studio.three-ds-connection";
const defaults: ThreeDsConnection = {
  address: "",
  port: DEFAULT_FTP_PORT,
  username: DEFAULT_FTP_USERNAME,
  password: "",
};

function read(): ThreeDsConnection {
  try {
    const raw = localStorage.getItem(storageKey);
    if (!raw) return { ...defaults };
    const parsed = JSON.parse(raw) as Partial<ThreeDsConnection>;
    return {
      address: typeof parsed.address === "string" ? parsed.address : "",
      port: typeof parsed.port === "string" ? parsed.port : defaults.port,
      username:
        typeof parsed.username === "string"
          ? parsed.username
          : defaults.username,
      password: typeof parsed.password === "string" ? parsed.password : "",
    };
  } catch {
    return { ...defaults };
  }
}

const stored = ref<ThreeDsConnection>(read());
watch(
  stored,
  (value) => {
    try {
      localStorage.setItem(storageKey, JSON.stringify(value));
    } catch {
      // Preferences are a convenience; a blocked store just means asking again.
    }
  },
  { deep: true },
);

export function useThreeDsConnection() {
  return {
    settings: readonly(stored),
    /** A trimmed address means wireless preparation and silent scans apply. */
    address: computed(() => stored.value.address.trim()),
    ftpPort: computed(() => {
      const port = Number(stored.value.port.trim() || DEFAULT_FTP_PORT);
      return Number.isInteger(port) && port > 0 && port < 65536 ? port : null;
    }),
    update(patch: Partial<ThreeDsConnection>): void {
      stored.value = { ...stored.value, ...patch };
    },
  };
}
