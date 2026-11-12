import test from "node:test";
import assert from "node:assert/strict";

import {
    ACCESS_TOKEN_COOKIE,
    ACCESS_TOKEN_MAX_AGE,
    buildAccessTokenCookieOps,
    serializeAccessTokenCookieOp
} from "../utils/accessTokenCookie.ts";

test("写入带共享域时先清 host-only 和 Domain，再只写一份新票", () => {
    const ops = buildAccessTokenCookieOps("new-token", ".example.com");
    assert.deepEqual(ops, [
        { type: "expire" },
        { type: "expire", domain: ".example.com" },
        { type: "set", value: "new-token", domain: ".example.com" }
    ]);

    const headers = ops.map(serializeAccessTokenCookieOp);
    assert.equal(headers[0].includes("Domain="), false);
    assert.equal(headers[0].includes("Max-Age=0"), true);
    assert.equal(headers[1].includes("Domain=.example.com"), true);
    assert.equal(headers[1].includes("Max-Age=0"), true);
    assert.equal(headers[2], `${ACCESS_TOKEN_COOKIE}=new-token; Max-Age=${ACCESS_TOKEN_MAX_AGE}; Path=/; SameSite=Lax; Domain=.example.com`);
});

test("无共享域时先清 host-only 再写入 30 天 Cookie", () => {
    const ops = buildAccessTokenCookieOps("new-token");
    assert.deepEqual(ops, [{ type: "expire" }, { type: "set", value: "new-token" }]);
    assert.equal(
        serializeAccessTokenCookieOp(ops[1]),
        `${ACCESS_TOKEN_COOKIE}=new-token; Max-Age=${ACCESS_TOKEN_MAX_AGE}; Path=/; SameSite=Lax`
    );
});

test("清除登录态时同时过期 host-only 和共享域 Cookie", () => {
    assert.deepEqual(buildAccessTokenCookieOps(null, ".example.com"), [
        { type: "expire" },
        { type: "expire", domain: ".example.com" }
    ]);
    assert.deepEqual(buildAccessTokenCookieOps(""), [{ type: "expire" }]);
});

test("token 含特殊字符时按 URI 编码写入", () => {
    const header = serializeAccessTokenCookieOp({ type: "set", value: "a+b/c=" });
    assert.equal(header.startsWith(`${ACCESS_TOKEN_COOKIE}=a%2Bb%2Fc%3D;`), true);
});
