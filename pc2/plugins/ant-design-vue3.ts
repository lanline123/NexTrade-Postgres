import Antd, { message } from "ant-design-vue";
import { ElMessage } from "element-plus";
import { isSilentError, SILENT_LOGIN_REDIRECT_MESSAGE } from "~/utils/request";

/**
 * 屏蔽 401 静默重定向带来的 toast：
 * 1) 入参是 SilentLoginRedirectError / 带 silent 标记的对象 → isSilentError 命中
 * 2) 入参是字符串 / { content } / { message }，且取值等于静默魔串 → 字符串兜底命中
 * 兜底分支用于兼容旧调用方写法（如 message.error(err.message)），保持向后兼容。
 */
const patchSilentMessage = <T extends (...args: any[]) => any>(handler: T): T => {
    return ((content: any, ...args: any[]) => {
        if (isSilentError(content)) {
            return;
        }
        const text = typeof content === "string" ? content : content?.content ?? content?.message;
        if (text === SILENT_LOGIN_REDIRECT_MESSAGE) {
            return;
        }
        return handler(content, ...args);
    }) as T;
};

export default defineNuxtPlugin((nuxtApp) => {
    message.error = patchSilentMessage(message.error);
    ElMessage.error = patchSilentMessage(ElMessage.error);
    nuxtApp.vueApp.use(Antd);
});
