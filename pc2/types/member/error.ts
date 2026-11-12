/**
 * 页面 catch 块中统一将 unknown 转为可展示文案
 */

export function errorMessage(error: unknown): string {
    if (error instanceof Error) return error.message;
    if (typeof error === "string") return error;
    if (typeof error === "object" && error !== null && "message" in error) {
        const m = (error as { message: unknown }).message;
        if (typeof m === "string") return m;
    }
    return "Unknown error";
}
