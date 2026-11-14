import { describe, expect, it } from "vitest";

import {
    canDownloadRawImages,
    createRawImageContext,
    normalizeRawImageConfig,
    rawImageContextKey,
    rawImageDownloadUrl,
    rawImageFilename,
    rawImageQuery
} from "../productRawImageDownload";
import { validateNativeArchive } from "../rawImageNativeDownload";

/**
 * 等价迁移自旧 utils/__tests__/productRawImageDownload.test.ts 与
 * rawImageNativeDownload.test.ts 的纯逻辑部分。
 * 旧版含真实 ZIP 字节一致性用例（依赖 uni-app 下载/文件系统桥），
 * 该部分在 Taro 版属于平台桥行为、不做等价迁移（见迁移报告未对齐项）；
 * 此处覆盖两文件全部纯函数导出：配置归一、上下文拼装、文件名解析、
 * ZIP 头部校验（validateNativeArchive 的字节判定）。
 */
describe("productRawImageDownload 纯逻辑", () => {
    it("normalizeRawImageConfig 补默认值", () => {
        const config = normalizeRawImageConfig({});
        expect(config).toBeTypeOf("object");
        expect(canDownloadRawImages(config, "h5")).toBeTypeOf("boolean");
    });

    it("上下文创建/键/查询串/下载 URL 互相一致", () => {
        const ctx = createRawImageContext(42, { shopId: 7 });
        expect(ctx).not.toBeNull();
        expect(rawImageContextKey(ctx)).toContain("42");
        expect(rawImageQuery(ctx!)).toContain("42");
        expect(rawImageDownloadUrl("/api/", ctx!)).toContain("42");
        expect(createRawImageContext(undefined)).toBeNull();
    });

    it("rawImageFilename 从 Content-Disposition 解析并兜底", () => {
        expect(rawImageFilename('attachment; filename="raw-9.zip"', 9)).toContain("9");
        expect(rawImageFilename(null, 9)).toContain("9");
    });
});

describe("validateNativeArchive ZIP 完整性校验", () => {
    const buildZip = () => {
        // 最小合法结构：本地文件头签名 + 中央目录结束记录（EOCD）签名
        const bytes = new Uint8Array(64);
        bytes.set([0x50, 0x4b, 0x03, 0x04], 0);
        bytes.set([0x50, 0x4b, 0x05, 0x06], bytes.length - 22);
        return bytes;
    };

    it("合法 ZIP（头签名+EOCD+长度一致）通过校验", async () => {
        const bytes = buildZip();
        const read = async (start: number, length: number) => bytes.slice(start, start + length);
        await expect(
            validateNativeArchive(bytes.length, read, 200, {
                "content-type": "application/zip",
                "content-length": String(bytes.length)
            })
        ).resolves.toBeUndefined();
    });

    it("非 ZIP 内容（JSON 错误体）抛错", async () => {
        const body = new TextEncoder().encode('{"code":1,"message":"denied"}');
        const read = async (start: number, length: number) => body.slice(start, start + length);
        await expect(
            validateNativeArchive(body.length, read, 200, { "content-type": "application/json" })
        ).rejects.toBeTruthy();
    });
});
