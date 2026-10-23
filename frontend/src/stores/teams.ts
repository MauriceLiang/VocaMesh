import { computed, ref } from "vue";
import { defineStore } from "pinia";
import type { Team, TeamMember } from "@/types";
import { ServiceError } from "@/types";
import { useServices } from "@/services/adapters";
import { useAppStore } from "./app";

export interface GraphPosition {
  x: number;
  y: number;
}

export const NODE_WIDTH = 196;
export const NODE_HEIGHT = 54;
const COLUMN_GAP = 236;
const ROW_GAP = 132;

/**
 * 团队 Store。
 *
 * 关系图坐标保存在这里（不写回 Rust）：正式实现可持久化为“视图偏好”，
 * 或每次按层级自动生成。组织关系本身由 team-service 负责。
 */
export const useTeamsStore = defineStore("teams", () => {
  const services = useServices();
  const app = useAppStore();

  const teams = ref<Team[]>([]);
  const selectedId = ref<string | null>(null);
  const memberFilter = ref("");
  const positions = ref<Record<string, GraphPosition>>({});
  const selectedMemberId = ref<string | null>(null);
  const loading = ref(false);
  const lastError = ref<string | null>(null);

  const selected = computed<Team | null>(() => teams.value.find((team) => team.id === selectedId.value) ?? null);
  const members = computed<TeamMember[]>(() => selected.value?.members ?? []);

  async function load(): Promise<void> {
    loading.value = true;
    try {
      teams.value = await services.teams.listTeams();
      lastError.value = null;
      if (!selectedId.value && teams.value.length) selectedId.value = teams.value[0].id;
      layout();
    } catch (err) {
      lastError.value = err instanceof ServiceError ? err.message : "无法读取本机团队配置，请重试。";
      app.toast(lastError.value, "failure");
    } finally {
      loading.value = false;
    }
  }

  function select(id: string): void {
    selectedId.value = id;
    selectedMemberId.value = null;
    layout();
  }

  function depthOf(team: Team, agentId: string): number {
    const byId = new Map(team.members.map((member) => [member.agentId, member]));
    let cursor = byId.get(agentId);
    let depth = 0;
    let guard = 0;
    while (cursor && cursor.agentId !== team.supervisorId && cursor.reportsTo && cursor.reportsTo !== "user" && guard++ < 40) {
      const parent = byId.get(cursor.reportsTo);
      if (!parent) break;
      cursor = parent;
      depth += 1;
    }
    return agentId === team.supervisorId ? 0 : depth;
  }

  /** 按层级生成初始坐标（用户拖动后以用户坐标为准）。 */
  function layout(force = false): void {
    const team = selected.value;
    if (!team) return;
    const groups = new Map<number, string[]>();
    team.members.forEach((member) => {
      if (!force && positions.value[member.agentId]) return;
      const depth = depthOf(team, member.agentId);
      const list = groups.get(depth) ?? [];
      list.push(member.agentId);
      groups.set(depth, list);
    });
    const maxCount = Math.max(1, ...[...groups.values()].map((list) => list.length));
    groups.forEach((ids, depth) => {
      const offset = ((maxCount - ids.length) * COLUMN_GAP) / 2;
      ids.forEach((id, index) => {
        positions.value[id] = { x: 40 + offset + index * COLUMN_GAP, y: 40 + depth * ROW_GAP };
      });
    });
    team.members.forEach((member) => {
      if (!positions.value[member.agentId]) positions.value[member.agentId] = { x: 40, y: 40 };
    });
  }

  function moveNode(agentId: string, position: GraphPosition): void {
    positions.value = { ...positions.value, [agentId]: position };
  }

  function selectMember(agentId: string | null): void {
    selectedMemberId.value = agentId;
  }

  async function apply(fn: () => Promise<Team>, successMessage?: string): Promise<boolean> {
    try {
      const team = await fn();
      teams.value = teams.value.map((item) => (item.id === team.id ? team : item));
      lastError.value = null;
      layout();
      if (successMessage) app.toast(successMessage, "success");
      return true;
    } catch (err) {
      const message = err instanceof ServiceError ? err.message : "操作失败";
      const detail = err instanceof ServiceError && err.detail ? `（${err.detail}）` : "";
      lastError.value = message;
      app.toast(message + detail, "blocked");
      return false;
    }
  }

  const currentId = () => selectedId.value ?? teams.value[0]?.id ?? "";

  async function addMember(agentId: string, reportsTo: string): Promise<boolean> {
    return apply(() => services.teams.addMember(currentId(), { agentId, reportsTo }), "已添加成员");
  }

  async function removeMember(agentId: string): Promise<boolean> {
    return apply(() => services.teams.removeMember(currentId(), agentId), "已从团队移除成员");
  }

  async function setReportsTo(agentId: string, reportsTo: string): Promise<boolean> {
    return apply(() => services.teams.setReportsTo(currentId(), agentId, reportsTo), "已更新汇报关系");
  }

  async function updateTeam(patch: Partial<Pick<Team, "name" | "supervisorId" | "workspace" | "workspaceId">>): Promise<boolean> {
    const current = selected.value;
    return apply(() => services.teams.updateTeam(currentId(), { ...patch, expectedRevision: current?.revision }), "团队设置已保存");
  }

  async function createTeam(input: { name: string; supervisorId: string; workspace: string; workspaceId?: string }): Promise<Team | null> {
    try {
      const team = await services.teams.createTeam(input);
      teams.value = [...teams.value, team];
      selectedId.value = team.id;
      layout();
      app.toast("团队已创建（关系检查通过）", "success");
      return team;
    } catch (err) {
      app.toast(err instanceof ServiceError ? err.message : "创建团队失败", "failure");
      return null;
    }
  }

  return {
    teams,
    selectedId,
    selected,
    members,
    memberFilter,
    positions,
    selectedMemberId,
    loading,
    lastError,
    load,
    select,
    layout,
    moveNode,
    selectMember,
    addMember,
    removeMember,
    setReportsTo,
    updateTeam,
    createTeam,
  };
});
