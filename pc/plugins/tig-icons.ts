/**
 * TigIcon 图标集注入插件
 *
 * 通过 import.meta.glob 批量加载 assets/icons/ 下的所有 SVG 文件为 Vue 组件，
 * 并通过 provide 注入到全局，供 TigIcon 组件使用。
 *
 * 注意：不带 .client 后缀，同时支持 SSR 和 CSR。
 */
const icons = import.meta.glob("~/assets/icons/**/*.svg", {
    eager: true,
    query: "?component"
});

export default defineNuxtPlugin((nuxtApp) => {
    nuxtApp.vueApp.provide("tig-icons", icons);
});
