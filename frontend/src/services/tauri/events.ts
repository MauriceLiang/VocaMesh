import { listen } from "@tauri-apps/api/event";
import type { EventService } from "@/services/contracts";

type ToolScanEvent = { phase: "scanning" | "done" | "failed"; message?: string };

export function createTauriToolScanEvents(): Pick<EventService, "subscribeToolScan"> {
  return {
    subscribeToolScan(handler) {
      let disposed = false;
      let unlisten: (() => void) | undefined;
      void listen<ToolScanEvent>("tool-scan", ({ payload }) => handler(payload))
        .then((stop) => {
          if (disposed) stop();
          else unlisten = stop;
        })
        .catch(() => {
          handler({ phase: "failed", message: "无法订阅本机扫描进度" });
        });
      return () => {
        disposed = true;
        unlisten?.();
      };
    },
  };
}
