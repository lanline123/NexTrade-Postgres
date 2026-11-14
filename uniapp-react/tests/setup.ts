/**
 * 测试环境全局桩：Taro 在构建期注入的宏函数（defineAppConfig 等）
 * 在 vitest 下不存在，而 src/utils/index.ts 会加载 app.config.ts，
 * 这里把宏补为恒等函数，使配置对象原样导出。
 */
(globalThis as any).defineAppConfig = (config: any) => config;
(globalThis as any).definePageConfig = (config: any) => config;
