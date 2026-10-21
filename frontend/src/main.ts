import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { router } from "./router";
import { useAppStore } from "./stores/app";
import "./styles/index.css";

/**
 * 应用启动：
 * 1) 先加载设置（主题/密度），避免首帧闪烁；
 * 2) 再挂载路由与外壳。
 * 说明：设置失败时使用默认值，不阻塞界面。
 */
async function bootstrap(): Promise<void> {
  const app = createApp(App);
  const pinia = createPinia();
  app.use(pinia);

  const appStore = useAppStore();
  try {
    await appStore.loadSettings();
  } catch (error) {
    console.warn("[vocamesh] 读取设置失败，使用默认外观", error);
  }

  app.use(router);
  app.mount("#app");
}

void bootstrap();
