/** Generated from contracts/store-v1. Run pnpm sync:store-protocol. */

export interface SignedCatalog {
  schema_version: 1;
  repository_id: string;
  channel: "stable";
  publication_id: string;
  sequence: number;
  parent: {
    publication_id: string;
    sha256: string;
  } | null;
  created_at: number;
  expires_at: number;
  apps: {
    id: string;
    slug: string;
    listing: "listed" | "unlisted";
    publisher: {
      id: string;
      name: string;
      verified: boolean;
    };
    category: "runtime" | "tool" | "app" | "game";
    source: {
      repository: string;
      path: string;
    };
    locales: {
      [k: string]: {
        name: string;
        summary: string;
        description: string;
      };
    };
    media: {
      role: "icon" | "screenshot";
      blob: {
        sha256: string;
        size_bytes: number;
        content_type: string;
      };
      locale: string | null;
      target_id: string | null;
      sort_order: number;
    }[];
  }[];
  releases: {
    id: string;
    app_id: string;
    version: string;
    revision: number;
    notes: {
      [k: string]: string;
    };
    published_at: number;
    status: "published" | "yanked";
    /**
     * @minItems 1
     */
    artifacts: [
      {
        id: string;
        format: string;
        blob: {
          sha256: string;
          size_bytes: number;
          content_type: string;
        };
        build_id: string;
        native_identity:
          | (
              | {
                  kind: "ios_bundle";
                  bundle_id: string;
                  version: string;
                  build_number: string;
                }
              | {
                  kind: "3ds_title";
                  title_id: string;
                  title_version: number;
                }
              | {
                  kind: "homebrew_file";
                  entrypoint: string;
                }
            )
          | null;
        provenance: {
          source_commit: string;
          runtime_commit: string | null;
          manifest_sha256: string;
        };
        /**
         * @minItems 1
         */
        targets: [
          {
            target_id: string;
            platform: string;
            arch: string;
            /**
             * @minItems 1
             */
            models: [string, ...string[]];
            os: {
              min: string;
              max: string | null;
              builds: string[];
            };
            host_abi: number | null;
            /**
             * @minItems 1
             */
            runtime_deliveries: [
              "bundled" | "shared",
              ...("bundled" | "shared")[],
            ];
            installer_id: string;
            requires:
              | {
                  kind: "ios";
                  jailbreak: boolean;
                  appsync: boolean;
                }
              | {
                  kind: "3ds";
                  cfw: boolean;
                };
            runtime_requirement: {
              id: string;
              min_version: string;
              bootstrap_app_id: string | null;
            } | null;
            runtime_provides: {
              id: string;
              version: string;
              capabilities: (
                | "guest-update"
                | "app-library"
                | "cia-management"
                | "file-management"
              )[];
            } | null;
          },
          ...{
            target_id: string;
            platform: string;
            arch: string;
            /**
             * @minItems 1
             */
            models: [string, ...string[]];
            os: {
              min: string;
              max: string | null;
              builds: string[];
            };
            host_abi: number | null;
            /**
             * @minItems 1
             */
            runtime_deliveries: [
              "bundled" | "shared",
              ...("bundled" | "shared")[],
            ];
            installer_id: string;
            requires:
              | {
                  kind: "ios";
                  jailbreak: boolean;
                  appsync: boolean;
                }
              | {
                  kind: "3ds";
                  cfw: boolean;
                };
            runtime_requirement: {
              id: string;
              min_version: string;
              bootstrap_app_id: string | null;
            } | null;
            runtime_provides: {
              id: string;
              version: string;
              capabilities: (
                | "guest-update"
                | "app-library"
                | "cia-management"
                | "file-management"
              )[];
            } | null;
          }[],
        ];
      },
      ...{
        id: string;
        format: string;
        blob: {
          sha256: string;
          size_bytes: number;
          content_type: string;
        };
        build_id: string;
        native_identity:
          | (
              | {
                  kind: "ios_bundle";
                  bundle_id: string;
                  version: string;
                  build_number: string;
                }
              | {
                  kind: "3ds_title";
                  title_id: string;
                  title_version: number;
                }
              | {
                  kind: "homebrew_file";
                  entrypoint: string;
                }
            )
          | null;
        provenance: {
          source_commit: string;
          runtime_commit: string | null;
          manifest_sha256: string;
        };
        /**
         * @minItems 1
         */
        targets: [
          {
            target_id: string;
            platform: string;
            arch: string;
            /**
             * @minItems 1
             */
            models: [string, ...string[]];
            os: {
              min: string;
              max: string | null;
              builds: string[];
            };
            host_abi: number | null;
            /**
             * @minItems 1
             */
            runtime_deliveries: [
              "bundled" | "shared",
              ...("bundled" | "shared")[],
            ];
            installer_id: string;
            requires:
              | {
                  kind: "ios";
                  jailbreak: boolean;
                  appsync: boolean;
                }
              | {
                  kind: "3ds";
                  cfw: boolean;
                };
            runtime_requirement: {
              id: string;
              min_version: string;
              bootstrap_app_id: string | null;
            } | null;
            runtime_provides: {
              id: string;
              version: string;
              capabilities: (
                | "guest-update"
                | "app-library"
                | "cia-management"
                | "file-management"
              )[];
            } | null;
          },
          ...{
            target_id: string;
            platform: string;
            arch: string;
            /**
             * @minItems 1
             */
            models: [string, ...string[]];
            os: {
              min: string;
              max: string | null;
              builds: string[];
            };
            host_abi: number | null;
            /**
             * @minItems 1
             */
            runtime_deliveries: [
              "bundled" | "shared",
              ...("bundled" | "shared")[],
            ];
            installer_id: string;
            requires:
              | {
                  kind: "ios";
                  jailbreak: boolean;
                  appsync: boolean;
                }
              | {
                  kind: "3ds";
                  cfw: boolean;
                };
            runtime_requirement: {
              id: string;
              min_version: string;
              bootstrap_app_id: string | null;
            } | null;
            runtime_provides: {
              id: string;
              version: string;
              capabilities: (
                | "guest-update"
                | "app-library"
                | "cia-management"
                | "file-management"
              )[];
            } | null;
          }[],
        ];
      }[],
    ];
  }[];
}
