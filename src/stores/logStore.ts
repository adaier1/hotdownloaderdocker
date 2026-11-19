import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

/**
 * 运行日志：解析、下载、账号与系统事件的流水。
 *
 * 只保留最近 MAX_ENTRIES 条，超出部分自动丢弃；持久化在本地
 * localStorage，Web 版与桌面端行为一致，且不依赖 Rust 侧存储。
 */

export const MAX_LOG_ENTRIES = 200

const STORAGE_KEY = 'hotdownloader-logs-v1'

export type LogLevel = 'success' | 'info' | 'warn' | 'error'
export type LogScope = 'parse' | 'download' | 'account' | 'system'

export interface LogEntry {
    id: string
    /** 事件发生时间（毫秒时间戳） */
    at: number
    level: LogLevel
    scope: LogScope
    /** 一句话结论，列表主文案 */
    title: string
    /** 可选的细节：失败原因、链接、音质、数量等 */
    detail?: string
    platform?: string
}

export type LogInput = Omit<LogEntry, 'id' | 'at'>

let sequence = 0

function createId(): string {
    sequence += 1
    return `${Date.now().toString(36)}-${sequence.toString(36)}`
}

/** 读取历史日志时逐条校验，避免旧版本或手工篡改的数据破坏页面。 */
function sanitize(raw: unknown): LogEntry[] {
    if (!Array.isArray(raw)) return []
    const levels: LogLevel[] = ['success', 'info', 'warn', 'error']
    const scopes: LogScope[] = ['parse', 'download', 'account', 'system']
    const entries: LogEntry[] = []
    for (const item of raw) {
        if (!item || typeof item !== 'object') continue
        const entry = item as Record<string, unknown>
        if (typeof entry.at !== 'number' || typeof entry.title !== 'string') continue
        entries.push({
            id: typeof entry.id === 'string' ? entry.id : createId(),
            at: entry.at,
            level: levels.includes(entry.level as LogLevel) ? entry.level as LogLevel : 'info',
            scope: scopes.includes(entry.scope as LogScope) ? entry.scope as LogScope : 'system',
            title: entry.title,
            detail: typeof entry.detail === 'string' ? entry.detail : undefined,
            platform: typeof entry.platform === 'string' ? entry.platform : undefined,
        })
    }
    // 始终按时间倒序，最多 200 条
    return entries.sort((a, b) => b.at - a.at).slice(0, MAX_LOG_ENTRIES)
}

export const useLogStore = defineStore('logs', () => {
    const entries = ref<LogEntry[]>([])
    const loaded = ref(false)

    let saveTimer: ReturnType<typeof setTimeout> | null = null

    function persist() {
        // 连续事件（例如批量入队）合并成一次写入，避免频繁序列化。
        if (saveTimer) clearTimeout(saveTimer)
        saveTimer = setTimeout(() => {
            saveTimer = null
            try {
                localStorage.setItem(STORAGE_KEY, JSON.stringify(entries.value))
            } catch (error) {
                console.warn('写入日志失败:', error)
            }
        }, 300)
    }

    function ensureLoaded() {
        if (loaded.value) return
        loaded.value = true
        try {
            const raw = localStorage.getItem(STORAGE_KEY)
            entries.value = sanitize(raw ? JSON.parse(raw) : [])
        } catch {
            entries.value = []
        }
    }

    /** 追加一条日志：新的在最前，超过上限的尾部自动丢弃。 */
    function push(input: LogInput): void {
        ensureLoaded()
        const entry: LogEntry = { ...input, id: createId(), at: Date.now() }
        const next = [entry, ...entries.value]
        entries.value = next.length > MAX_LOG_ENTRIES ? next.slice(0, MAX_LOG_ENTRIES) : next
        persist()
    }

    function clear(): void {
        entries.value = []
        if (saveTimer) {
            clearTimeout(saveTimer)
            saveTimer = null
        }
        try {
            localStorage.removeItem(STORAGE_KEY)
        } catch (error) {
            console.warn('清空日志失败:', error)
        }
    }

    /** 最近 n 条，供总览页使用。 */
    function recent(count: number): LogEntry[] {
        ensureLoaded()
        return entries.value.slice(0, count)
    }

    const errorCount = computed(() => entries.value.filter((e) => e.level === 'error').length)

    const scopeCounts = computed(() => {
        const counts = { all: entries.value.length, parse: 0, download: 0, account: 0, system: 0 }
        for (const entry of entries.value) {
            counts[entry.scope] += 1
        }
        return counts
    })

    // 首次创建 store 时就读取本地记录，避免首帧渲染时列表为空再闪一次。
    ensureLoaded()

    return { entries, errorCount, scopeCounts, ensureLoaded, push, clear, recent }
})
