import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ExecutionService, TaskService, RunService, EventService } from "@/services/contracts";
import { ServiceError } from "@/types";
import type { ExecutionReadiness, LogLine, NativeRunEvent, NativeRunRecord, NativeTaskRecord, RecoveryInspection, RecoveryResolution, RunArtifactGroup, RunRecord, StartReadOnlyTaskInput, Task, UnknownRunSummary } from "@/types";

interface IpcError { code: string; message: string }
interface RunPage { items: NativeRunRecord[]; hasMore: boolean; nextCursor: number | null }
interface StartResponse { task: NativeTaskRecord; run: NativeRunRecord }
const MAX_VISIBLE_EVENTS = 500;

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try { return await invoke<T>(command, args); }
  catch (error) {
    if (error && typeof error === "object" && "code" in error && "message" in error) {
      const ipc = error as IpcError;
      if (typeof ipc.code === "string" && typeof ipc.message === "string") throw new ServiceError(ipc.message, ipc.code);
    }
    throw new ServiceError("本机执行服务暂时不可用", "execution.ipc-failed");
  }
}

export function createTauriExecutionService(): ExecutionService {
  return {
    selectWorkspaceDirectory: () => call<string | null>("select_workspace_directory"),
    readiness: (providerId) => call<ExecutionReadiness>("get_execution_readiness", { providerId }),
    listUnknownRuns: () => call<UnknownRunSummary[]>("list_unknown_runs"),
    inspectUnknownRun: (runId) => call<RecoveryInspection>("inspect_unknown_run", { runId }),
    resolveUnknownRun: (runId, userConfirmed) => call<RecoveryResolution>("resolve_unknown_run", { runId, userConfirmed }),
    startReadOnlyTask: (input: StartReadOnlyTaskInput) => call<StartResponse>("start_read_only_task", { request: input }),
    stopRun: (runId) => call<NativeRunRecord>("stop_run", { runId }),
    getRun: (runId) => call<NativeRunRecord>("get_run", { runId }),
    getTask: (taskId) => call<NativeTaskRecord>("get_task", { taskId }),
    listTasks: () => call<NativeTaskRecord[]>("list_tasks"),
    listRuns: (cursor, limit) => call<RunPage>("list_runs", { cursor, limit }),
    listRunEvents: (runId, after, limit) => call<NativeRunEvent[]>("list_run_events", { runId, after, limit }),
    subscribeRunState(handler) {
      let cancelled = false;
      let unlisten: (() => void) | undefined;
      void listen<NativeRunRecord>("run://state", (event) => handler(event.payload)).then((stop) => {
        if (cancelled) stop(); else unlisten = stop;
      }).catch(() => undefined);
      return () => { cancelled = true; unlisten?.(); };
    },
    subscribeRunEvents(runId, handler) {
      let cancelled = false;
      let ready = false;
      let unlisten: (() => void) | undefined;
      let buffered: NativeRunEvent[] = [];
      let lastSequence = 0;
      const publish = (events: NativeRunEvent[]) => {
        events.sort((left, right) => left.sequence - right.sequence);
        for (const event of events) {
          if (event.sequence <= lastSequence) continue;
          lastSequence = event.sequence;
          handler(event);
        }
      };
      void listen<NativeRunEvent>("run://event", (event) => {
        if (event.payload.runId !== runId) return;
        if (ready) publish([event.payload]);
        else buffered.push(event.payload);
      }).then(async (stop) => {
        if (cancelled) { stop(); return; }
        unlisten = stop;
        const history: NativeRunEvent[] = [];
        try {
          const run = await call<NativeRunRecord>("get_run", { runId });
          let cursor = Math.max(0, run.lastSequence - MAX_VISIBLE_EVENTS);
          while (cursor < run.lastSequence) {
            const page = await call<NativeRunEvent[]>("list_run_events", { runId, after: cursor, limit: 100 });
            if (!page.length) break;
            history.push(...page);
            const next = page[page.length - 1].sequence;
            if (next <= cursor) break;
            cursor = next;
          }
        } catch { /* the live stream remains usable if historical replay fails */ }
        const pending = buffered;
        buffered = [];
        publish([...history, ...pending]);
        ready = true;
      }).catch(() => { ready = true; });
      return () => { cancelled = true; unlisten?.(); };
    },
  };
}

function mapTask(record: NativeTaskRecord): Task {
  const state = mapTaskState(record.state);
  const stateNote = ({
    queued: "等待本机 Claude Code 执行",
    starting: "正在启动本机 Claude Code",
    running: "Claude Code 正在只读分析工作区快照",
    "stop-pending": "正在停止，等待进程树退出确认",
    stopped: "进程已停止",
    done: "只读分析已完成",
    failed: "运行失败，查看 Run 日志",
    unknown: "执行状态未知，需要检查历史记录",
  } as Record<string, string>)[state] ?? "本机任务状态";
  return {
    id: record.id,
    name: record.title,
    workspace: record.workspacePath,
    mode: "single-agent",
    team: "Claude Code · 单 Agent",
    runId: record.currentRunId,
    supervisor: "Claude Code",
    createdAt: new Date(record.createdAt).toISOString(),
    startedAt: ["queued", "starting"].includes(state) ? "未开始" : new Date(record.updatedAt).toISOString(),
    state,
    stateNote,
    subtasks: [],
    messages: [],
    nativeExecution: true,
    prompt: record.prompt,
  };
}

function mapTaskState(state: string): Task["state"] {
  if (["queued", "starting", "running", "stop-pending", "stopped", "done", "failed", "unknown"].includes(state)) return state as Task["state"];
  return "unknown";
}

function mapRun(record: NativeRunRecord, taskName: string): RunRecord {
  const started = record.startedAt;
  const ended = record.finishedAt ?? (started == null ? null : Date.now());
  const duration = started == null || ended == null ? "—" : `${Math.max(0, Math.round((ended - started) / 1000))} 秒`;
  return {
    id: record.id,
    taskId: record.taskId,
    taskName,
    executor: "Claude Code · 本机只读",
    state: record.outcome === "stopped" ? "stopped" : record.outcome === "success" ? "finished" : ["failed", "timeout", "spawn-failed", "output-protocol-error", "missing-final-result", "unknown"].includes(record.outcome ?? "") ? (record.outcome === "unknown" ? "unknown" : "crashed") : record.state,
    startedAt: started == null ? "待启动" : new Date(started).toLocaleString(),
    duration,
    providerId: record.providerId,
    outcome: record.outcome,
    summary: record.summary,
    outputTruncated: record.outputTruncated,
    errorCode: record.errorCode,
    finishedAt: record.finishedAt == null ? null : new Date(record.finishedAt).toLocaleString(),
  };
}

export function createTauriTaskService(execution: ExecutionService): TaskService {
  const unsupported = () => Promise.reject(new ServiceError("本阶段只支持只读任务停止；暂停、恢复、重试和部分成果尚未实现", "task.operation-unsupported"));
  return {
    async listTasks() { return (await execution.listTasks()).map(mapTask); },
    async getTask(id) { return mapTask(await execution.getTask(id)); },
    async createTask() { throw new ServiceError("桌面端请使用“开始只读任务”创建真实任务", "execution.consent-required"); },
    async pauseTask() { return unsupported(); },
    async resumeTask() { return unsupported(); },
    async stopTask(id) {
      const task = await execution.getTask(id);
      await execution.stopRun(task.currentRunId);
      return mapTask(await execution.getTask(id));
    },
    async retryFailed() { return unsupported(); },
    async skipAndFinalizePartial() { return unsupported(); },
    async listSubtasks(taskId) { return mapTask(await execution.getTask(taskId)).subtasks; },
    subscribeTaskState(taskId, handler) {
      return execution.subscribeRunState((run) => {
        if (run.taskId === taskId) void execution.getTask(taskId).then((task) => handler(mapTask(task).state, mapTask(task)));
      });
    },
  };
}

export function createTauriRunService(execution: ExecutionService): RunService {
  const taskNames = async () => new Map((await execution.listTasks()).map((task) => [task.id, task.title]));
  const matchesResult = (run: NativeRunRecord, result?: string) => {
    if (!result || result === "全部") return true;
    if (result === "已完成") return run.outcome === "success";
    if (result === "失败") return run.outcome !== "success" && run.outcome !== "stopped" && run.outcome !== "unknown";
    if (result === "已停止") return run.outcome === "stopped";
    if (result === "状态未知") return run.outcome === "unknown";
    return false;
  };
  return {
    async listRuns(filter) {
      const [page, names] = await Promise.all([execution.listRuns(filter?.cursor ? Number(filter.cursor) : undefined, 50), taskNames()]);
      const filtered = page.items.filter((run) => matchesResult(run, filter?.result));
      const items = filtered.map((run) => mapRun(run, names.get(run.taskId) ?? "本地任务"));
      return { items, hasMore: page.hasMore, nextCursor: page.nextCursor == null ? null : String(page.nextCursor) };
    },
    async getPartialArtifacts(_runId): Promise<RunArtifactGroup[]> { return []; },
    async listArtifacts() { return []; },
    async exportRunsCsv(filter) {
      const names = await taskNames();
      const records: NativeRunRecord[] = [];
      let cursor: number | undefined;
      let more = true;
      while (more) {
        const page = await execution.listRuns(cursor, 100);
        records.push(...page.items.filter((run) => matchesResult(run, filter?.result)));
        cursor = page.nextCursor ?? undefined;
        more = page.hasMore;
      }
      const rows = ["run_id,task,provider,state,outcome,started_at,exit_code", ...records.map((run) => [run.id, names.get(run.taskId) ?? "本地任务", run.providerId, run.state, run.outcome ?? "", run.startedAt ?? "", run.exitCode ?? ""].map(csv).join(","))];
      return rows.join("\n");
    },
  };
}

function csv(value: string | number): string { return `"${String(value).replaceAll('"', '""')}"`; }

export function createTauriRuntimeEvents(execution: ExecutionService, subscribeToolScan: EventService["subscribeToolScan"]): EventService {
  return {
    subscribeTaskLogs(runId, handler) {
      const deliver = (event: NativeRunEvent) => {
        handler(toLog(event));
      };
      return execution.subscribeRunEvents(runId, deliver);
    },
    subscribeTaskState(handler) {
      return execution.subscribeRunState((run) => {
        void execution.getTask(run.taskId).then((task) => handler(mapTask(task)));
      });
    },
    subscribeApprovals() { return () => undefined; },
    subscribeToolScan,
  };
}

function toLog(event: NativeRunEvent): LogLine {
  const level: LogLine["level"] = event.kind === "protocol-error" ? "ERROR" : event.kind === "truncated" ? "WARN" : "INFO";
  return { time: new Date(event.timestamp).toLocaleTimeString(), level, actor: "Claude Code", message: event.text, eventKind: event.kind, sequence: event.sequence };
}
