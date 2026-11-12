const assert = require("node:assert/strict");
const test = require("node:test");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const ts = require("typescript");

// Load only the real transport module; every Nuxt, store and network dependency is isolated.
function loadRequest(options = {}) {
    const calls = [];
    const redirects = [];
    const messages = [];
    const logs = [];
    const user = {
        token: options.token ?? "test-token",
        isAuthenticated: Boolean(options.token ?? "test-token"),
        clearToken() { this.token = ""; this.isAuthenticated = false; }
    };
    const payload = options.payload ?? { code: 0, message: "ok", data: { id: 7 } };
    const dependencies = {
        "element-plus": { ElMessage: { error: (value) => messages.push(value) } },
        "~/components/verifition/utils/ase": { aesEncrypt: () => "fixture-signature" },
        "~/utils/util": { getCurrentRoute: () => ({ fullPath: "/product/7" }), formatArguments: (value) => value },
        "~/store/user": { useUserStore: () => user },
        "~/composables/useRequestLocaleCode": { useRequestLocaleCode: () => ({ value: "en" }) }
    };
    const module = { exports: {} };
    const context = {
        module, exports: module.exports,
        require(name) {
            assert.ok(Object.hasOwn(dependencies, name), `Unexpected import: ${name}`);
            return dependencies[name];
        },
        __env: { VITE_API_URL: "https://fixture.invalid", VITE_API_PREFIX: "/api" },
        __client: false,
        tryUseNuxtApp: () => ({}),
        useCookie: () => ({ value: "en" }),
        navigateTo: (value) => redirects.push(value),
        console: { error: (value) => logs.push(value) },
        async useFetch(url, config) {
            calls.push({ url, config });
            return { data: { value: options.empty ? null : payload }, error: { value: options.ssrError ?? null } };
        },
        async $fetch(url, config) {
            calls.push({ url, config });
            if (options.httpError) config.onResponseError({ response: { _data: options.httpError } });
            if (options.clientError) throw options.clientError;
            return payload;
        }
    };
    const file = path.join(__dirname, "../utils/request.ts");
    const source = fs.readFileSync(file, "utf8")
        .replaceAll("import.meta.env", "__env")
        .replaceAll("import.meta.client", "__client");
    const { outputText } = ts.transpileModule(source, {
        fileName: file,
        compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS }
    });
    vm.runInNewContext(outputText, context, { filename: file });
    return { ...module.exports, calls, redirects, messages, logs, user };
}

for (const method of ["request", "asyncRequest"]) {
    for (const data of [null, undefined, [], { records: [1], total: 1 }, { data: { moduleList: [] } }]) {
        test(`${method} unwraps exactly one envelope: ${JSON.stringify(data)}`, async () => {
            const client = loadRequest({ payload: { code: 0, message: "ok", data } });
            assert.equal(await client[method]({ url: "/fixture", method: "get" }), data);
        });
    }

    test(`${method} preserves business error objects`, async () => {
        const payload = { code: 9, message: "fixture error", data: { field: "name" } };
        const client = loadRequest({ payload });
        await assert.rejects(client[method]({ url: "/fixture", method: "get" }), (error) => error === payload);
    });

    test(`${method} keeps login redirects silent`, async () => {
        const client = loadRequest({ payload: { code: 401, message: "unauthorized", data: null } });
        await assert.rejects(client[method]({ url: "/fixture", method: "get" }), (error) => {
            assert.equal(client.isSilentError(error), true);
            assert.equal(error.redirectLogin, true);
            return true;
        });
        assert.deepEqual(client.redirects, ["/member/login?returnUrl=/product/7"]);
        assert.equal(client.user.token, "");
        assert.equal(client.logs.length, 0);
    });

    test(`${method} preserves anonymous noSkipLogin behavior`, async () => {
        const payload = { code: 401, message: "unauthorized", data: null };
        const client = loadRequest({ payload, token: "" });
        await assert.rejects(client[method]({ url: "/fixture", method: "get", noSkipLogin: true }), (error) => error === payload);
        assert.deepEqual(client.redirects, []);
    });

    test(`${method} preserves POST body, prefix and custom headers`, async () => {
        const client = loadRequest();
        await client[method]({ url: "/fixture", method: "post", prefix: "/custom", data: { id: 7 }, headers: { Custom: "value" } });
        const { config } = client.calls[0];
        assert.equal(config.baseURL, "https://fixture.invalid/custom");
        assert.equal(config.body, JSON.stringify({ id: 7 }));
        assert.equal(config.params, "");
        assert.equal(config.headers.Authorization, "Bearer test-token");
        assert.equal(config.headers["X-Locale-Code"], "en");
        assert.equal(config.headers.Custom, "value");
    });
}

test("SSR preserves nested useFetch error data", async () => {
    const failure = { code: 17, message: "network payload" };
    const client = loadRequest({ ssrError: { data: { data: failure } } });
    await assert.rejects(client.request({ url: "/fixture", method: "get" }), (error) => error === failure);
});

test("SSR preserves the empty-response fallback", async () => {
    const client = loadRequest({ empty: true });
    await assert.rejects(client.request({ url: "/fixture", method: "get" }), (error) => error.errcode === 1);
});

test("client accepts legacy serialized JSON envelopes", async () => {
    const client = loadRequest({ payload: JSON.stringify({ code: 0, data: { id: 7 } }) });
    const result = await client.asyncRequest({ url: "/fixture", method: "get" });
    assert.equal(result.id, 7);
    assert.equal(Object.hasOwn(result, "code"), false);
});

test("client preserves transport failure and HTTP error notification", async () => {
    const failure = new Error("offline");
    const client = loadRequest({ clientError: failure, httpError: { code: 500, msg: "fixture server error" } });
    await assert.rejects(client.asyncRequest({ url: "/fixture", method: "get" }), (error) => error === failure);
    assert.deepEqual(client.messages, ["fixture server error"]);
    assert.equal(client.calls[0].config.lazy, true);
    assert.equal(client.calls[0].config.server, false);
});
