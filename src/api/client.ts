// Web 版 API 客户端：替代 Tauri 的 `invoke` / `listen`。
//
// - `invoke(cmd, args)` -> `POST /api/invoke`（统一 RPC 分发，约定与 Tauri invoke 一致）
// - `listen(event, handler)` -> 复用一条 SSE 连接 `/api/events`，按事件名分发
//
// nginx 会把 `/api` 反向代理到 Rust API 服务，因此前端与后端同源，无跨域问题。
// 本地开发时由 vite 的 server.proxy 完成代理。

const API_BASE = (import.meta.env.VITE_API_BASE as string | undefined) ?? '/api'

interface InvokeResponse<T> {
    code: number
    data: T
    error: string | null
}

/**
 * 调用后端命令。
 *
 * @param cmd 命令名（与 Rust 端 dispatch 保持一致）
 * @param args 参数对象（camelCase）
 */
export async function invoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    // 通知权限由浏览器管理，直接在本地处理，无需请求服务端
    if (cmd === 'request_notification_permission') {
        if (typeof Notification === 'undefined') return false as unknown as T
        if (Notification.permission === 'granted') return true as unknown as T
        const result = await Notification.requestPermission()
        return (result === 'granted') as unknown as T
    }
    if (cmd === 'check_notification_permission') {
        const granted = typeof Notification !== 'undefined' && Notification.permission === 'granted'
        return granted as unknown as T
    }

    const resp = await fetch(`${API_BASE}/invoke`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ cmd, args: args ?? {} }),
    })
    if (!resp.ok) {
        throw new Error(`请求失败: HTTP ${resp.status}`)
    }
    const body = (await resp.json()) as InvokeResponse<T>
    if (body.code !== 0) {
        throw new Error(body.error || '命令执行失败')
    }
    return body.data
}

/** 取消事件监听的函数 */
export type UnlistenFn = () => void

type Handler = (event: { payload: any }) => void

const listeners = new Map<string, Set<Handler>>()
const boundEvents = new Set<string>()
let eventSource: EventSource | null = null

function bindEvent(name: string) {
    if (!eventSource || boundEvents.has(name)) return
    boundEvents.add(name)
    eventSource.addEventListener(name, (ev) => {
        const raw = (ev as MessageEvent).data
        let payload: unknown = raw
        try {
            payload = JSON.parse(raw)
        } catch {
            // 非 JSON 数据原样透传
        }
        listeners.get(name)?.forEach((handler) => handler({ payload }))
    })
}

function ensureEventSource() {
    if (eventSource) return
    eventSource = new EventSource(`${API_BASE}/events`)
    eventSource.onerror = () => {
        // EventSource 会自动重连（服务端会回放最近事件），这里仅提示
        console.warn('[HotDownloader] SSE 连接中断，浏览器将自动重连')
    }
    for (const name of listeners.keys()) bindEvent(name)
}

/**
 * 监听后端 SSE 事件。
 *
 * @param event 事件名（download-progress / download-completed / ...）
 * @param handler 事件回调，`event.payload` 为后端推送的数据
 * @returns 取消监听的函数
 */
export function listen<T = unknown>(
    event: string,
    handler: (event: { payload: T }) => void
): Promise<UnlistenFn> {
    ensureEventSource()
    if (!listeners.has(event)) listeners.set(event, new Set())
    listeners.get(event)!.add(handler as Handler)
    bindEvent(event)

    return Promise.resolve(() => {
        const set = listeners.get(event)
        set?.delete(handler as Handler)
        if (set && set.size === 0) listeners.delete(event)
    })
}

/** 触发浏览器下载服务端下载目录中的文件 */
export function downloadServerFile(fileName: string): void {
    window.open(`${API_BASE}/files?name=${encodeURIComponent(fileName)}`, '_blank')
}
