import { invoke, isTauri } from '@tauri-apps/api/core'
import { webRequest } from './webClient'

export interface NeteaseLoginResult {
    logged_in?: boolean
    userid?: string
    nickname?: string
    qrcode_id?: string
    qr_image?: string
    status?: 'waiting' | 'scanned' | 'confirmed' | 'expired' | 'error'
}

export function neteaseLogin(
    action: 'status' | 'qr' | 'check' | 'manual' | 'refresh' | 'logout',
    id?: string,
    credentials?: { cookie: string },
): Promise<NeteaseLoginResult> {
    const args = { action, id, credentials }
    return isTauri()
        ? invoke('netease_login', args)
        : webRequest('/api/login/netease', {
              method: 'POST',
              body: JSON.stringify(args),
          })
}
