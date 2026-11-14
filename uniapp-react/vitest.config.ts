import path from "path";
import { defineConfig } from "vitest/config";

/**
 * 测试基座（补齐旧版 vitest 配置的等价物）。
 * 旧版：jsdom + globals + include src/**\/*.{test,spec}.{js,ts} + alias @→src，
 * 另有剥离 pages.json 注释的自研插件（uni-app 特有，Taro 版不需要）。
 * 本版差异：globals 关闭、测试文件显式 import vitest 符号（避免改 tsconfig types）；
 * @tarojs/taro 别名到 tests/mocks/taro.ts 桩，使 store/utils/hooks 可在 jsdom 下加载。
 */
export default defineConfig({
    resolve: {
        alias: [
            { find: /^@tarojs\/taro$/, replacement: path.resolve(__dirname, "tests/mocks/taro.ts") },
            { find: /^@\/(.*)$/, replacement: path.resolve(__dirname, "src/$1") }
        ]
    },
    test: {
        environment: "jsdom",
        setupFiles: [path.resolve(__dirname, "tests/setup.ts")],
        include: ["src/**/*.{test,spec}.{js,ts,tsx}"],
        exclude: ["node_modules", "dist"],
        testTimeout: 10000,
        hookTimeout: 10000
    }
});
