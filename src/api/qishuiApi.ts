import { invoke, isTauri } from '@tauri-apps/api/core'
import { webRequest } from './webClient'

/** 汽水 Cookie 状态：只含是否配置与键名摘要，不含 Cookie 值。 */
export interface QishuiCookieStatus {
    configured: boolean
    length: number
    keys: string
}

/** 汽水签名器状态：只含可用性与路径摘要，不含任何凭据。 */
export interface QishuiSignerStatus {
    available: boolean
    /** `local`（本机 bdms.node）/ `remote`（远程签名服务）/ 空字符串。 */
    mode: string
    bdms: string
    runtime: string
    signerUrl: string
    electronMode: boolean
    deviceId: string
}

const native = isTauri()

export async function getQishuiSignerStatus(): Promise<QishuiSignerStatus> {
    if (native) {
        return invoke<QishuiSignerStatus>('get_qishui_signer_status')
    }
    return webRequest<QishuiSignerStatus>('/api/qishui/signer')
}

export async function getQishuiCookieStatus(): Promise<QishuiCookieStatus> {
    if (native) {
        return invoke<QishuiCookieStatus>('get_qishui_cookie_status')
    }
    return webRequest<QishuiCookieStatus>('/api/qishui/cookie')
}

export async function saveQishuiCookie(cookie: string): Promise<QishuiCookieStatus> {
    if (native) {
        return invoke<QishuiCookieStatus>('save_qishui_cookie', { cookie })
    }
    return webRequest<QishuiCookieStatus>('/api/qishui/cookie', {
        method: 'POST',
        body: JSON.stringify({ cookie }),
    })
}

export async function clearQishuiCookie(): Promise<void> {
    if (native) {
        await invoke('clear_qishui_cookie')
        return
    }
    await webRequest('/api/qishui/cookie', { method: 'DELETE' })
}
