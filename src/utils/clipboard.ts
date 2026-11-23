/**
 * 剪贴板写入工具。
 *
 * 浏览器只在「安全上下文」（HTTPS 或 localhost）下暴露 navigator.clipboard，
 * 而 WebUI 常被以 `http://<局域网 IP>:<端口>` 的方式访问，此时 Clipboard API
 * 直接不可用（undefined），调用即抛错。这里按「Clipboard API → execCommand」
 * 的顺序回退，保证 HTTP 访问下也能复制。
 */

function execCommandCopy(text: string): boolean {
    if (typeof document === 'undefined' || typeof document.execCommand !== 'function') {
        return false
    }

    const active = document.activeElement as HTMLElement | null
    const selection = typeof document.getSelection === 'function' ? document.getSelection() : null
    const savedRanges: Range[] = []
    if (selection) {
        for (let i = 0; i < selection.rangeCount; i += 1) {
            savedRanges.push(selection.getRangeAt(i))
        }
    }

    const textarea = document.createElement('textarea')
    textarea.value = text
    textarea.setAttribute('readonly', '')
    textarea.setAttribute('aria-hidden', 'true')
    // 固定在左上角并缩到最小，避免 iOS 聚焦时页面跳动或触发缩放
    textarea.style.position = 'fixed'
    textarea.style.top = '0'
    textarea.style.left = '0'
    textarea.style.width = '1px'
    textarea.style.height = '1px'
    textarea.style.padding = '0'
    textarea.style.border = 'none'
    textarea.style.outline = 'none'
    textarea.style.boxShadow = 'none'
    textarea.style.background = 'transparent'
    textarea.style.opacity = '0'
    textarea.style.fontSize = '12pt'

    document.body.appendChild(textarea)

    let copied = false
    try {
        textarea.focus({ preventScroll: true })
        textarea.select()
        textarea.setSelectionRange(0, text.length)
        // iOS Safari 对 readonly 元素的 select() 不可靠，补一次 Range 选区
        if (selection) {
            const range = document.createRange()
            range.selectNodeContents(textarea)
            selection.removeAllRanges()
            selection.addRange(range)
        }
        copied = document.execCommand('copy')
    } catch {
        copied = false
    } finally {
        textarea.remove()
        // 还原此前的焦点与选区，避免影响用户正在操作的输入框
        active?.focus?.({ preventScroll: true })
        if (selection) {
            selection.removeAllRanges()
            for (const range of savedRanges) {
                selection.addRange(range)
            }
        }
    }

    return copied
}

/**
 * 复制文本，成功返回 true；失败时由调用方回退到「选中文本 + 提示手动复制」。
 */
export async function copyText(text: string): Promise<boolean> {
    if (!text) return false

    if (typeof navigator !== 'undefined' && navigator.clipboard && window.isSecureContext) {
        try {
            await navigator.clipboard.writeText(text)
            return true
        } catch {
            // 权限被拒绝或文档未聚焦时继续尝试回退方案
        }
    }

    return execCommandCopy(text)
}
