// 分享链接的前端即时识别。
// 与后端 `platforms::link_parse` 规则保持一致，只做提示，最终结果仍以服务端解析为准。

export type ShareLinkKind = 'song' | 'playlist' | 'album' | 'unknown'

export interface ShareLinkHint {
    platform: string
    kind: ShareLinkKind
}

export const PLATFORM_NAMES: Record<string, string> = {
    qqmusic: 'QQ 音乐',
    kuwo: '酷我音乐',
    kugou: '酷狗音乐',
    netease: '网易云音乐',
    bilibili: '哔哩哔哩',
    qishui: '汽水音乐',
}

export const LINK_KIND_NAMES: Record<ShareLinkKind, string> = {
    song: '单曲',
    playlist: '歌单',
    album: '专辑',
    unknown: '待确认',
}

const URL_PATTERN = /https?:\/\/[^\s<>"'）】」》，。！？、]+/

/** 从整段分享文案里取出第一个链接。 */
export function extractShareUrl(text: string): string | null {
    return URL_PATTERN.exec(text)?.[0] ?? null
}

function hostOf(url: string): string | null {
    try {
        return new URL(url).hostname.toLowerCase().replace(/\.$/, '')
    } catch {
        return null
    }
}

/** 按 host 精确匹配平台，不接受「文本里出现某域名」这种宽松判断。 */
export function detectSharePlatform(url: string): string | null {
    const host = hostOf(url)
    if (!host) return null
    const bare = host.replace(/^www\./, '')
    if (['music.163.com', 'y.music.163.com', '163cn.tv', '163cn.com'].includes(bare)) return 'netease'
    if (host.endsWith('y.qq.com')) return 'qqmusic'
    if (bare === 'kugou.com' || host.endsWith('.kugou.com')) return 'kugou'
    if (bare === 'kuwo.cn' || host.endsWith('.kuwo.cn')) return 'kuwo'
    if (host.endsWith('bilibili.com') || host === 'b23.tv') return 'bilibili'
    // 汽水分享短链与抖音同域，只认 qishui 子域与 /qishui/ 分享页。
    if (bare === 'qishui.douyin.com') return 'qishui'
    if (bare === 'music.douyin.com' && url.toLowerCase().includes('/qishui/')) return 'qishui'
    return null
}

function routeOf(url: string): string {
    try {
        const parsed = new URL(url)
        const route = `${parsed.pathname}${parsed.search}${parsed.hash}`
        return decodeURIComponent(route).toLowerCase()
    } catch {
        return url.toLowerCase()
    }
}

function classify(route: string, playlist: string[], album: string[], song: string[]): ShareLinkKind {
    if (playlist.some((marker) => route.includes(marker))) return 'playlist'
    if (album.some((marker) => route.includes(marker))) return 'album'
    if (song.some((marker) => route.includes(marker))) return 'song'
    return 'unknown'
}

/** 判断链接指向的内容类型；短链离线无法确定，返回 unknown 由歌单路径兜底。 */
export function detectShareKind(platform: string, url: string): ShareLinkKind {
    const host = hostOf(url) ?? ''
    const route = routeOf(url)
    switch (platform) {
        case 'netease':
            if (host.endsWith('163cn.tv') || host.endsWith('163cn.com')) return 'unknown'
            return classify(route, ['/playlist'], ['/album'], ['/song'])
        case 'qqmusic':
            return classify(route, ['playlist', 'taoge'], ['albumdetail'], ['songdetail', 'songmid'])
        case 'kugou':
            return classify(route, ['/special/single', 'global_collection_id'], ['/album/'], ['/song/', '/mixsong/'])
        case 'kuwo':
            return classify(route, ['/playlist_detail', '/playlist/'], ['/album_detail', '/album/'], ['/play_detail', '/play/'])
        // B 站只有视频，BV 号本身就是单曲。
        case 'bilibili':
            return 'song'
        // 汽水分享短链离线无法区分单曲与歌单，按单曲处理；歌单在后端给出明确错误。
        case 'qishui':
            if (route.includes('/share/playlist') || route.includes('playlist_id')) return 'playlist'
            if (route.includes('/share/album')) return 'album'
            return 'song'
        default:
            return 'unknown'
    }
}

/** 输入变化时给出一行即时反馈；没有链接或平台不支持时返回 null。 */
export function detectShareHint(text: string): ShareLinkHint | null {
    const url = extractShareUrl(text)
    if (!url) return null
    const platform = detectSharePlatform(url)
    if (!platform) return null
    return { platform, kind: detectShareKind(platform, url) }
}
