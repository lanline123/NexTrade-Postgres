/**
 * 页面导航 模块（渲染端 registry 独有 type，编辑器新注册；
 * 默认值字段对齐 uniapp-react 渲染端 customPage/modules/navbar/index.tsx 读取的 module 字段）
 *
 * 渲染端读取：title（标题文字）、titleColor（标题颜色）、titleBackgroundColor（导航背景色）。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep } from "../../../contract/sharedDefaults";

/** 与渲染端 navbar/index.tsx 读取字段逐项对应 */
export const navbarDefaults = {
    title: "页面标题",
    titleColor: "#333333",
    titleBackgroundColor: "#ffffff"
};

registerModule({
    type: "navbar",
    label: "页面导航",
    icon: '<i class="h5-mod-ico">🧭</i>',
    url: "mobile",
    platform: "mobile",
    order: 17,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(navbarDefaults)
});
