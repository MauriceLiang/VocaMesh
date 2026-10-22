import type { ExecutionReadiness, NativeRunEvent, NativeRunRecord, NativeTaskRecord, StartReadOnlyTaskInput } from "@/types";

export interface ExecutionService {
  selectWorkspaceDirectory(): Promise<string | null>;
  readiness(): Promise<ExecutionReadiness>;
  startReadOnlyTask(input: StartReadOnlyTaskInput): Promise<{ task: NativeTaskRecord; run: NativeRunRecord }>;
  stopRun(runId: string): Promise<NativeRunRecord>;
  getRun(runId: string): Promise<NativeRunRecord>;
  getTask(taskId: string): Promise<NativeTaskRecord>;
  listTasks(): Promise<NativeTaskRecord[]>;
  listRuns(cursor?: number, limit?: number): Promise<{ items: NativeRunRecord[]; hasMore: boolean; nextCursor: number | null }>;
  listRunEvents(runId: string, after?: number, limit?: number): Promise<NativeRunEvent[]>;
  subscribeRunState(handler: (run: NativeRunRecord) => void): () => void;
  subscribeRunEvents(runId: string, handler: (event: NativeRunEvent) => void): () => void;
}
