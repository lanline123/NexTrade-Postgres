import { test } from "node:test";
import assert from "node:assert/strict";
import {
    normalizeRawImageConfig, canDownloadRawImages, createRawImageContext, rawImageContextKey, rawImageQuery, rawImageDownloadUrl,
    rawImageFilename, validateRawImageBlob, fetchRawImageArchive, validRawImageIntent
} from "../utils/productRawImageDownload.ts";

// A real ZIP fixture containing main_001.png (three original bytes).
const zip = Buffer.from("UEsDBBQAAAAAAE23U10dgLxVAwAAAAMAAAAMAAAAbWFpbl8wMDEucG5nAQIDUEsBAhQDFAAAAAAATbdTXR2AvFUDAAAAAwAAAAwAAAAAAAAAAAAAAIABAAAAAG1haW5fMDAxLnBuZ1BLBQYAAAAAAQABADoAAAAtAAAAAAA=", "base64");
const blob = () => new Blob([zip], { type: "application/zip" });

test("missing/unknown config fails closed, explicit empty choices remain empty", () => {
    assert.equal(canDownloadRawImages({}, "pc"), false);
    assert.deepEqual(normalizeRawImageConfig({ productRawImageDownloadChannels: [] }).productRawImageDownloadChannels, []);
    assert.equal(canDownloadRawImages({ productRawImageDownloadEnabled: "1", productRawImageDownloadChannels: ["pc"] }, "pc"), false);
});
for (const [client, channel] of Object.entries({ pc: "pc", h5: "h5", wechat: "h5", miniProgram: "miniProgram", app: "app", android: "app", ios: "app" })) {
    test(`channel ${client} maps to ${channel}`, () => {
        const config = { productRawImageDownloadEnabled: 1, productRawImageDownloadChannels: [channel] };
        assert.equal(canDownloadRawImages(config, client), true);
        for (const unknown of ["pos", "", "mp-alipay", "PC", "constructor", "__proto__"]) assert.equal(canDownloadRawImages(config, unknown), false);
    });
}
test("download uses numeric product ID and captured request context, including zero", () => {
    const params = { shopId: 0, nearestShopId: 8, sn: "abc", token: "do-not-send", skuId: 99 };
    const context = createRawImageContext("42", params);
    params.shopId = 123;
    assert.deepEqual(context, { id: 42, shopId: 0, nearestShopId: 8 });
    assert.equal(rawImageQuery(context), "id=42&shopId=0&nearestShopId=8");
    assert.deepEqual(createRawImageContext(42, { shopId: -1, nearestShopId: null }), { id: 42 });
    for (const id of [0, -1, null, undefined, "", "abc", "1.2", true, Infinity]) assert.equal(createRawImageContext(id), null);
});
test("UTF-8 disposition wins and malformed encoding falls back safely", () => {
    assert.equal(rawImageFilename("attachment; filename=images.zip; filename*=UTF-8''%E5%95%86%E5%93%81.zip", 42), "商品.zip");
    assert.equal(rawImageFilename("attachment; filename=images.zip; filename*=UTF-8''%broken", 42), "images.zip");
    assert.equal(rawImageFilename(null, 42), "product_42_raw_images.zip");
    assert.equal(rawImageFilename('attachment; filename="../../x\\y.zip"', 42).includes("/"), false);
    assert.equal(rawImageFilename('attachment; filename="bad\r\nname.zip"', 42).includes("\n"), false);
});
test("real ZIP remains byte-identical and valid", async () => {
    const value = blob();
    await validateRawImageBlob(value, 200, "application/zip", String(zip.length));
    assert.deepEqual(Buffer.from(await value.arrayBuffer()), zip);
});
for (const code of [401, 1001]) {
    test(`HTTP 200 JSON ${code} is never a downloadable ZIP`, async () => {
        await assert.rejects(validateRawImageBlob(new Blob([JSON.stringify({ code, message: "服务端提示" })]), 200, "application/json"),
            (error) => error.code === code && error.message === "服务端提示" && error.serverMessage);
    });
}
test("JSON with ZIP MIME, HTML, empty, partial and wrong-length responses are rejected", async () => {
    for (const value of [new Blob(['{"code":401,"message":"登录"}']), new Blob(["<html>error</html>"]), new Blob([]), new Blob([zip.subarray(0, 50)])]) {
        await assert.rejects(validateRawImageBlob(value, 200, "application/zip"));
    }
    await assert.rejects(validateRawImageBlob(blob(), 200, "application/zip", "999"));
    await assert.rejects(validateRawImageBlob(blob(), 500, "application/zip"));
    await assert.rejects(validateRawImageBlob(blob(), 200, "text/html"));
});
test("login intent expires and is restricted to one route and exact shop context", () => {
    const context = { id: 42, shopId: 0 };
    const intent = { route: "/item/abc", context: rawImageContextKey(context), expires: 2000 };
    assert.equal(validRawImageIntent(intent, "/item/abc", context, 1000), true);
    assert.equal(validRawImageIntent(intent, "/item/abc", { id: 42, shopId: 2 }, 1000), false);
    assert.equal(validRawImageIntent(intent, "/item/other", context, 1000), false);
    assert.equal(validRawImageIntent(intent, "/item/abc", context, 2000), false);
});
test("binary transport preserves auth headers, omits cookies and refuses redirects", async () => {
    const original = globalThis.fetch;
    const controller = new AbortController();
    const headers = { Authorization: "Bearer test-token", Secret: "test-secret", "X-Client-Type": "pc", "X-Locale-Code": "zh_CN" };
    globalThis.fetch = async (url, options) => {
        assert.equal(url, "/api/product/product/downloadRawImages?id=42");
        assert.deepEqual(options.headers, headers);
        assert.equal(options.signal, controller.signal);
        assert.equal(options.redirect, "error");
        assert.equal(options.credentials, "omit");
        return new Response(blob(), { headers: { "content-type": "application/zip", "content-disposition": "attachment; filename=images.zip" } });
    };
    try {
        const result = await fetchRawImageArchive("/api/product/product/downloadRawImages?id=42", headers, controller.signal, 42);
        assert.equal(result.filename, "images.zip");
        assert.deepEqual(Buffer.from(await result.blob.arrayBuffer()), zip);
    } finally { globalThis.fetch = original; }
});

test("API prefixes with or without a trailing slash resolve the same endpoint", () => {
    for (const prefix of ["/api", "/api/"]) assert.equal(rawImageDownloadUrl(prefix, { id: 42, shopId: 0 }), "/api/product/product/downloadRawImages?id=42&shopId=0");
    assert.equal(rawImageDownloadUrl("https://example.test/api/", { id: 42 }), "https://example.test/api/product/product/downloadRawImages?id=42");
});
