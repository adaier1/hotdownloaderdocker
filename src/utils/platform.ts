/**
 * 浏览器平台识别（替代 Tauri 的 `@tauri-apps/plugin-os`）。
 *
 * Web 版运行在浏览器中，这里基于 User-Agent 粗略判断平台，
 * 用于更新检查时过滤对应平台的安装包等场景。
 */
export async function platform(): Promise<string> {
    if (typeof navigator === 'undefined') return 'web'
    const ua = navigator.userAgent.toLowerCase()
    if (ua.includes('android')) return 'android'
    if (ua.includes('windows')) return 'windows'
    if (ua.includes('mac os') || ua.includes('macintosh')) return 'macos'
    if (ua.includes('linux')) return 'linux'
    return 'web'
}
