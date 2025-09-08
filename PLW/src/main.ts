import "./../node_modules/bulma/css/bulma.min.css";

import { createApp } from "vue";
import App from "@/App.vue";

const app = createApp(App);
app.provide(
  "worker",
  new Worker(new URL("@/lib/worker.ts", import.meta.url), { type: "module" }),
);

app.mount("#app");
