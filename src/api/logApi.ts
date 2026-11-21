import { isTauri } from '@tauri-apps/api/core'
import { webRequest } from './webClient'

/**
 * 运行日志的存取边界。
 *
 * - 浏览器/独立服务：日志保存在服务端 `data/logs.json`（环形缓冲），
 *   换设备、清理浏览器缓存或更换访问端口后记录仍在。
 * - 桌面端：没有独立服务进程，沿用本机 localStorage。
 */

export const MAX_LOG_ENTRIES = 200

/** 桌面端沿用旧版本写入的键，升级后历史日志不丢。 */
const NATIVE_STORAGE_KEY = 'hotdownloader-logs-v1'

export type LogLevel = 'success' | 'info' | 'warn' | 'error'
export type LogScope = 'parse' | 'download' | 'account' | 'system'

export interface LogEntry {
    id: string
    /** 事件发生时间（毫秒时间戳），由服务端落库时生成。 */
    at: number
    level: LogLevel
    scope: LogScope
    /** 一句话结论，列表主文案 */
    title: string
    /** 可选的细节：失败原因、链接、音质、数量等 */
    detail?: string
    platform?: string
}

/** 提交字段：页面可先生成 id，服务端据此幂等写入，回显不会重复。 */
export type LogInput = Omit<LogEntry, 'at' | 'id'> & { id?: string }

const LEVELS: LogLevel[] = ['success', 'info', 'warn', 'error']
const SCOPES: LogScope[] = ['parse', 'download', 'account', 'system']

/** 读取本地记录时逐条校验，避免旧版本或手工篡改的数据破坏页面。 */
function sanitize(raw: unknown): LogEntry[] {
    if (!Array.isArray(raw)) return []
    const entries: LogEntry[] = []
    for (const item of raw) {
        if (!item || typeof item !== 'object') continue
        const entry = item as Record<string, unknown>
        if (typeof entry.at !== 'number' || typeof entry.title !== 'string') continue
        entries.push({
            id: typeof entry.id === 'string' ? entry.id : `${entry.at}`,
            at: entry.at,
            level: LEVELS.includes(entry.level as LogLevel) ? entry.level as LogLevel : 'info',
            scope: SCOPES.includes(entry.scope as LogScope) ? entry.scope as LogScope : 'system',
            title: entry.title,
            detail: typeof entry.detail === 'string' ? entry.detail : undefined,
            platform: typeof entry.platform === 'string' ? entry.platform : undefined,
        })
    }
    // 始终按时间倒序，最多 MAX_LOG_ENTRIES 条
    return entries.sort((a, b) => b.at - a.at).slice(0, MAX_LOG_ENTRIES)
}

function readNative(): LogEntry[] {
    try {
        const raw = localStorage.getItem(NATIVE_STORAGE_KEY)
        return sanitize(raw ? JSON.parse(raw) : [])
    } catch {
        return []
    }
}

function writeNative(entries: LogEntry[]): void {
    try {
        localStorage.setItem(NATIVE_STORAGE_KEY, JSON.stringify(entries))
    } catch (error) {
        // 隐私模式或配额用尽时只影响本机日志，不阻断页面。
        console.warn('写入日志失败:', error)
    }
}

/** 读取全部日志（新的在前）。 */
export async function fetchLogs(): Promise<LogEntry[]> {
    if (isTauri()) return readNative()
    return webRequest<LogEntry[]>('/api/logs')
}

/** 追加一条日志，返回落库后的记录（服务端统一分配时间戳）。 */
export async function appendLog(entry: LogEntry): Promise<LogEntry> {
    if (isTauri()) {
        const stored: LogEntry = { ...entry, at: Date.now() }
        const next = [stored, ...readNative().filter(item => item.id !== entry.id)]
        writeNative(next.slice(0, MAX_LOG_ENTRIES))
        return stored
    }
    // 服务端只接受 id 与内容字段，`at` 由服务端落库时生成。
    const { at: _ignored, ...payload } = entry
    return webRequest<LogEntry>('/api/logs', {
        method: 'POST',
        body: JSON.stringify(payload),
    })
}

/** 清空全部日志（不可恢复）。 */
export async function clearLogs(): Promise<void> {
    if (isTauri()) {
        try {
            localStorage.removeItem(NATIVE_STORAGE_KEY)
        } catch (error) {
            console.warn('清空日志失败:', error)
        }
        return
    }
    await webRequest<{ ok: boolean }>('/api/logs/clear', { method: 'POST' })
}
