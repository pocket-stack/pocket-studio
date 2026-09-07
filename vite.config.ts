import { fileURLToPath } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import vue from "@vitejs/plugin-vue";
import { FileSystemIconLoader } from "unplugin-icons/loaders";
import IconsResolver from "unplugin-icons/resolver";
import Icons from "unplugin-icons/vite";
import Components from "unplugin-vue-components/vite";
import { defineConfig } from "vite";
import svgLoader from "vite-svg-loader";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [
    vue(),
    // Preserve the drawings' CSS-driven states and animation styles.
    svgLoader({ svgo: false }),
    Icons({
      compiler: "vue3",
      customCollections: {
        studio: FileSystemIconLoader(
          fileURLToPath(new URL("./src/assets/icons", import.meta.url)),
        ),
      },
      defaultClass: "shrink-0",
      iconCustomizer(_collection, _icon, props) {
        props.width = "18";
        props.height = "18";
        props["aria-hidden"] = "true";
        props.focusable = "false";
      },
    }),
    Components({
      dirs: [],
      dts: "src/components.d.ts",
      types: [],
      resolvers: [
        IconsResolver({
          prefix: "Icon",
          enabledCollections: [],
          customCollections: ["studio"],
        }),
      ],
    }),
    tailwindcss(),
  ],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
