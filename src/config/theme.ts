import type { GlobalThemeOverrides } from 'naive-ui'

// Apple 风格浅色主题：与 style.css 中的设计令牌保持一致。
export const lightThemeOverrides: GlobalThemeOverrides = {
    common: {
        fontFamily:
            '-apple-system, BlinkMacSystemFont, "SF Pro Display", "SF Pro Text", "Helvetica Neue", Helvetica, "Segoe UI", "PingFang SC", "Microsoft YaHei", Arial, sans-serif',
        fontSize: '14px',
        fontSizeSmall: '13px',
        fontSizeMedium: '14px',
        borderRadius: '8px',
        borderRadiusSmall: '6px',
        primaryColor: '#0071e3',
        primaryColorHover: '#0077ed',
        primaryColorPressed: '#006edb',
        primaryColorSuppl: '#0071e3',
        infoColor: '#0071e3',
        successColor: '#34c759',
        warningColor: '#ff9500',
        errorColor: '#ff3b30',
        bodyColor: '#f5f5f7',
        cardColor: '#ffffff',
        modalColor: '#ffffff',
        popoverColor: '#ffffff',
        tableColor: '#ffffff',
        inputColor: '#ffffff',
        borderColor: '#d2d2d7',
        dividerColor: '#e8e8ed',
        textColor1: '#1d1d1f',
        textColor2: '#86868b',
        textColor3: '#aeaeb2',
        placeholderColor: '#aeaeb2',
        hoverColor: 'rgba(0, 0, 0, 0.04)',
    },
    Button: {
        borderRadiusTiny: '6px',
        borderRadiusSmall: '8px',
        borderRadiusMedium: '10px',
        borderRadiusLarge: '12px',
        fontWeight: '500',
    },
    Input: {
        borderRadius: '8px',
        border: '1px solid #d2d2d7',
        borderHover: '1px solid #aeaeb2',
        borderFocus: '1px solid #0071e3',
        boxShadowFocus: '0 0 0 3px rgba(0, 113, 227, 0.12)',
        color: '#ffffff',
        colorFocus: '#ffffff',
    },
    InputNumber: {
        borderRadius: '8px',
    },
    Select: {
        peers: {
            InternalSelection: {
                borderRadius: '8px',
                border: '1px solid #d2d2d7',
                borderHover: '1px solid #aeaeb2',
                borderActive: '1px solid #0071e3',
                borderFocus: '1px solid #0071e3',
                boxShadowActive: '0 0 0 3px rgba(0, 113, 227, 0.12)',
                boxShadowFocus: '0 0 0 3px rgba(0, 113, 227, 0.12)',
            },
        },
    },
    Switch: {
        railColorActive: '#34c759',
    },
    Card: {
        borderRadius: '12px',
        color: '#ffffff',
        borderColor: '#e8e8ed',
    },
    Modal: {
        color: '#ffffff',
    },
    Tag: {
        borderRadius: '12px',
    },
    Checkbox: {
        borderRadius: '5px',
    },
    Progress: {
        railColor: '#e8e8ed',
    },
}
