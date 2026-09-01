import { createApp } from "vue";

import App from "./App.vue";
import { createAppI18n } from "./shared/i18n";
import { initializeTheme } from "./shared/theme";
import "./styles/main.css";

initializeTheme();

createApp(App).use(createAppI18n()).mount("#app");
