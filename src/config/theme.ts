import type { GlobalThemeOverrides } from 'naive-ui'

// MusicDock 风格浅色主题：与 style.css 中的设计令牌保持一致。
export const lightThemeOverrides: GlobalThemeOverrides = {
    common: {
        fontFamily:
            '"Noto Sans SC", -apple-system, BlinkMacSystemFont, "SF Pro Display", "SF Pro Text", "Helvetica Neue", Helvetica, "Segoe UI", "PingFang SC", "Microsoft YaHei", Arial, sans-serif',
        fontSize: '14px',
        fontSizeSmall: '13px',
        fontSizeMedium: '14px',
        borderRadius: '8px',
        borderRadiusSmall: '6px',
        primaryColor: '#2b6bf3',
        primaryColorHover: '#1f55c8',
        primaryColorPressed: '#1a49ad',
        primaryColorSuppl: '#2b6bf3',
        infoColor: '#2b6bf3',
        successColor: '#16a34a',
        warningColor: '#d97706',
        errorColor: '#ff4d4f',
        bodyColor: '#f5f6f8',
        cardColor: '#ffffff',
        modalColor: '#ffffff',
        popoverColor: '#ffffff',
        tableColor: '#ffffff',
        inputColor: '#ffffff',
        borderColor: '#e6e8eb',
        dividerColor: '#eef0f3',
        textColor1: '#1f2329',
        textColor2: '#5f6672',
        textColor3: '#9aa1ab',
        placeholderColor: '#9aa1ab',
        hoverColor: 'rgba(0, 0, 0, 0.04)',
    },
    Button: {
        borderRadiusTiny: '6px',
        borderRadiusSmall: '8px',
        borderRadiusMedium: '9px',
        borderRadiusLarge: '12px',
        fontWeight: '500',
    },
    Input: {
        borderRadius: '8px',
        border: '1px solid #e6e8eb',
        borderHover: '1px solid #dde1e6',
        borderFocus: '1px solid #2b6bf3',
        boxShadowFocus: '0 0 0 3px rgba(43, 107, 243, 0.12)',
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
                border: '1px solid #e6e8eb',
                borderHover: '1px solid #dde1e6',
                borderActive: '1px solid #2b6bf3',
                borderFocus: '1px solid #2b6bf3',
                boxShadowActive: '0 0 0 3px rgba(43, 107, 243, 0.12)',
                boxShadowFocus: '0 0 0 3px rgba(43, 107, 243, 0.12)',
            },
        },
    },
    Switch: {
        railColorActive: '#2b6bf3',
    },
    Card: {
        borderRadius: '10px',
        color: '#ffffff',
        borderColor: '#e6e8eb',
    },
    Modal: {
        color: '#ffffff',
    },
    Tag: {
        borderRadius: '6px',
    },
    Checkbox: {
        borderRadius: '5px',
    },
    Progress: {
        railColor: '#eef0f3',
    },
}
