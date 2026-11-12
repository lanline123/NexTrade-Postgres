/**
 * 将不可信文本进行 HTML 转义，避免在 v-html/innerHTML 场景发生 XSS。
 * 注意：如果你要输出“我们自己拼出来的安全 HTML”，应先对用户输入 escape，再插入白名单标签。
 */
export function escapeHtml(input: unknown): string {
    if (input === null || input === undefined) return "";
    const str = String(input);
    const map: Record<string, string> = {
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#39;"
    };
    return str.replace(/[&<>"']/g, (m) => map[m] ?? m);
}


