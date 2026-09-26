import { lightThemeOverrides } from '../config/theme'

/** 统一提供 Naive UI 的组件样式配置（Apple 风格浅色主题）。 */
export function useAppTheme() {
    return {
        theme: null,
        themeOverrides: lightThemeOverrides,
    }
}
