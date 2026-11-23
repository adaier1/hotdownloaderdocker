/**
 * 剪贴板写入工具。
 *
 * 浏览器只在「安全上下文」（HTTPS 或 localhost）下暴露 navigator.clipboard，
 * 而 WebUI 常被以 `http://<局域网 IP>:<端口>` 的方式访问，此时 Clipboard API
 * 直接不可用（undefined）。这里按「Clipboard API → execCommand」顺序回退。
 *
 * 回退分支的关键点：textarea 的 value 不是子节点，
 * `range.selectNodeContents(textarea)` 得到的是空选区——此时
 * execCommand('copy') 仍会返回 true，但剪贴板是空的。所以 textarea 分支只
 * 依赖 select() + setSelectionRange()，绝不动 document 选区；只有 iOS 的
 * contenteditable 分支才用 Range。
 */

function isAppleMobile(): boolean {
    if (typeof navigator === 'undefined') return false
    const ua = navigator.userAgent || ''
    if (/ipad|iphone|ipod/i.test(ua)) return true
    // iPadOS 13+ 的 UA 伪装成 macOS，用触点数区分
    return /macintosh|mac os x/i.test(ua) && (navigator.maxTouchPoints ?? 0) > 1
}

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

    // iOS 对 readonly textarea 的 select() 不可靠，只能走 contenteditable + Range
    const useEditable = isAppleMobile()
    const field = useEditable ? null : document.createElement('textarea')
    const editable = useEditable ? document.createElement('div') : null
    const holder: HTMLElement = (field ?? editable) as HTMLElement

    if (field) {
        field.value = text
        field.setAttribute('readonly', '')
    } else if (editable) {
        editable.textContent = text
        editable.contentEditable = 'true'
    }

    // 移出视口而不是 opacity:0，保证选区在真机上有效
    const style = holder.style
    style.position = 'absolute'
    style.left = '-9999px'
    style.top = '0'
    style.width = '2em'
    style.height = '2em'
    style.margin = '0'
    style.padding = '0'
    style.border = 'none'
    style.outline = 'none'
    style.boxShadow = 'none'
    style.background = 'transparent'
    style.fontSize = '12pt'
    style.lineHeight = '1'
    style.overflow = 'hidden'
    holder.setAttribute('aria-hidden', 'true')
    holder.setAttribute('tabindex', '-1')

    document.body.appendChild(holder)

    let copied = false
    try {
        holder.focus({ preventScroll: true })
        if (field) {
            // 只操作控件自身的选区，不要碰 document 选区
            field.select()
            field.setSelectionRange(0, text.length)
        } else if (editable && selection) {
            const range = document.createRange()
            range.selectNodeContents(editable)
            selection.removeAllRanges()
            selection.addRange(range)
            // iOS 需要再用输入框 API 校正一次，否则粘贴出来可能是空的
            editable.focus({ preventScroll: true })
        }
        copied = document.execCommand('copy')
    } catch {
        copied = false
    } finally {
        holder.remove()
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
