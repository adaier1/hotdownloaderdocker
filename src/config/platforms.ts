// 平台配置列表，用于前端平台切换按钮和 API 调用。
// 每个平台包含 key（后端标识）和 label（显示缩写）。
export interface PlatformOption {
    key: string
    label: string
}

export const PLATFORMS: PlatformOption[] = [
  { key: 'qqmusic', label: 'QQ' },
  { key: 'kuwo', label: 'KW' },
  { key: 'kugou', label: 'KG' },
  { key: 'netease', label: 'WY' },
  { key: 'qishui', label: 'QS' },
]

// 搜索页可选平台：汽水没有可用的搜索接口（需要本机签名，公网部署拿不到），
// 只保留分享链接解析入口，因此不列在这里。
export const SEARCH_PLATFORMS: PlatformOption[] = PLATFORMS.filter((item) => item.key !== 'qishui')

// 默认平台 key
export const DEFAULT_PLATFORM = PLATFORMS[0].key