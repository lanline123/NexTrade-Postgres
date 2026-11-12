// i18n-ignore-next-line 用于比对后端返回的 message 原文，不是展示文案
const INVALID_DECORATE_PAGE_MESSAGE = "模板不存在";

function getErrorMessage(error: unknown): string | undefined {
    if (!error || typeof error !== "object" || !("message" in error)) {
        return undefined;
    }

    const { message } = error as { message?: unknown };
    return typeof message === "string" ? message : undefined;
}

export async function handleDecorateLoadError(error: unknown): Promise<boolean> {
    if (getErrorMessage(error) !== INVALID_DECORATE_PAGE_MESSAGE) {
        return false;
    }

    await navigateTo("/404");
    return true;
}
