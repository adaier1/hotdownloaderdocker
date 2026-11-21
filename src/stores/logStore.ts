import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import {
    MAX_LOG_ENTRIES,
    appendLog,
    clearLogs,
    fetchLogs,
    type LogEntry,
    type LogInput,
} from '../api/logApi'

/**
 * 运行日志：解析、下载、账号与系统事件的流水。
 *
 * 浏览器端读取服务端 `data/logs.json` 的环形缓冲，桌面端写在本地
 * localStorage；页面只负责乐观插入与渲染，落库细节由 logApi 处理。
 */

export { MAX_LOG_ENTRIES }
export type { LogEntry, LogInput, LogLevel, LogScope } from '../api/logApi'

let sequence = 0

function createId(): string {
    sequence += 1
    return `${Date.now().toString(36)}-${sequence.toString(36)}`
}

export const useLogStore = defineStore('logs', () => {
    const entries = ref<LogEntry[]>([])
    /** 是否已经从存储读过一次，避免每次进入页面都重新请求。 */
    const loaded = ref(false)
    const syncing = ref(false)
    const error = ref('')

    /** 按 id 合并：同一 id 覆盖，新 id 按时间插入，保持新的在前。 */
    function merge(entry: LogEntry) {
        const list = entries.value
        const index = list.findIndex(item => item.id === entry.id)
        if (index >= 0) {
            list[index] = entry
            return
        }
        const insertAt = list.findIndex(item => item.at <= entry.at)
        if (insertAt < 0) {
            list.push(entry)
        } else {
            list.splice(insertAt, 0, entry)
        }
        if (list.length > MAX_LOG_ENTRIES) {
            list.length = MAX_LOG_ENTRIES
        }
    }

    /** 从存储读取完整列表；`force` 用于页面激活时刷新其他设备写入的记录。 */
    async function load(force = false): Promise<void> {
        if (syncing.value) return
        if (loaded.value && !force) return
        syncing.value = true
        try {
            entries.value = await fetchLogs()
            loaded.value = true
            error.value = ''
        } catch (cause) {
            error.value = cause instanceof Error ? cause.message : String(cause)
        } finally {
            syncing.value = false
        }
    }

    /** 追加一条日志：先乐观插入，再提交存储；失败时保留本地记录并提示。 */
    function push(input: LogInput): void {
        const entry: LogEntry = {
            ...input,
            id: input.id || createId(),
            at: Date.now(),
        }
        merge(entry)
        void appendLog(entry)
            .then(saved => { if (saved?.id) merge(saved) })
            .catch((cause: unknown) => {
                console.warn('写入日志失败:', cause)
                error.value = cause instanceof Error ? cause.message : String(cause)
            })
    }

    /** 其他页面写入的日志通过 SSE 到达。 */
    function applyRemote(entry: LogEntry): void {
        merge(entry)
    }

    /** 其他页面清空日志时同步本页。 */
    function applyCleared(): void {
        entries.value = []
    }

    async function clear(): Promise<void> {
        const previous = entries.value
        entries.value = []
        error.value = ''
        try {
            await clearLogs()
        } catch (cause) {
            // 清空失败时恢复列表，避免界面显示与服务端不一致。
            entries.value = previous
            error.value = cause instanceof Error ? cause.message : String(cause)
        }
    }

    /** 最近 n 条，供总览页使用。 */
    function recent(count: number): LogEntry[] {
        return entries.value.slice(0, count)
    }

    /** 兼容旧调用点：首次使用时补一次读取。 */
    function ensureLoaded(): void {
        void load()
    }

    const errorCount = computed(() => entries.value.filter((e) => e.level === 'error').length)

    const scopeCounts = computed(() => {
        const counts = { all: entries.value.length, parse: 0, download: 0, account: 0, system: 0 }
        for (const entry of entries.value) {
            counts[entry.scope] += 1
        }
        return counts
    })

    return {
        entries,
        loaded,
        syncing,
        error,
        errorCount,
        scopeCounts,
        load,
        ensureLoaded,
        push,
        applyRemote,
        applyCleared,
        clear,
        recent,
    }
})
