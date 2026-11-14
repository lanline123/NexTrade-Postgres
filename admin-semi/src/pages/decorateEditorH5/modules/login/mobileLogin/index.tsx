/**
 * 登录入口 模块（契约依据：contract/sharedDefaults.defaultMobileLogin 现成默认值，直接复用；
 * 渲染端 pages/login/index.tsx 取 moduleList 中 type=mobileLogin 的 module，BaseLogin.tsx 按
 * titleColor/subTextColor/inputTextColor/placeholderColor/linkColor/buttonColor/dividerColor/
 * contentPaddingX/contentPaddingTop 下发 CSS 变量驱动登录页）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultMobileLogin } from "../../../contract/sharedDefaults";

export const mobileLoginDefaults = cloneDeep(defaultMobileLogin);

registerModule({
    type: "mobileLogin",
    label: "登录入口",
    icon: '<i class="h5-mod-ico">🔑</i>',
    url: "mobileLogin",
    platform: "mobile",
    order: 102,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(mobileLoginDefaults)
});
