import { invoke, isTauri } from '@tauri-apps/api/core'
import { webRequest } from './webClient'

export interface KugouCredentials {
    userid: string
    token: string
    mid: string
    dfid: string
}

export interface KugouLoginResult {
    logged_in?: boolean
    userid?: string
    qrcode_id?: string
    qr_image?: string
    status?: 'waiting' | 'scanned' | 'confirmed' | 'expired'
}

export function kugouLogin(
    action: 'status' | 'qr' | 'check' | 'manual' | 'logout',
    id?: string,
    credentials?: KugouCredentials,
): Promise<KugouLoginResult> {
    const args = { action, id, credentials }
    return isTauri()
        ? invoke('kugou_login', args)
        : webRequest('/api/login/kugou', {
              method: 'POST',
              body: JSON.stringify(args),
          })
}
