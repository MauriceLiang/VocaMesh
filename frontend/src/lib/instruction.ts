import type { InstructionState } from "@/types";
import type { StatusMeta } from "./status";

/** 定向指令投递状态 → 展示语义（与原型一致）。 */
export const instructionStateMeta = (state: InstructionState): StatusMeta => {
  switch (state) {
    case "delivered": return { label: "已送达（运行时确认）", tone: "neutral", icon: "dot" };
    case "applied": return { label: "已应用", tone: "success", icon: "check" };
    case "not-applied": return { label: "未应用", tone: "failure", icon: "x" };
    case "waiting-point": return { label: "等待安全投递点", tone: "blocked", icon: "alert" };
    case "failed": return { label: "投递失败", tone: "failure", icon: "x" };
    case "running": return { label: "执行中", tone: "neutral", icon: "loader" };
    default: return { label: "排队待投递", tone: "neutral", icon: "hourglass" };
  }
};
