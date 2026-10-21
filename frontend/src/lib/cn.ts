import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

/** 组合 Tailwind 类名（与原型的手写类不冲突，仅用于工具类布局）。 */
export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}
