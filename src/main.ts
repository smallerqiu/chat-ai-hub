import { createApp } from "vue";
import Kui from "kui-vue";
import "kui-vue/style/index.css";
import App from "./App.vue";
import "./styles.css";

createApp(App).use(Kui).mount("#root");
