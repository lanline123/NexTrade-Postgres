/**
 * 店铺订单卡 模块（mobilePro）
 *
 * 注意：storeOrderCard 在 uniapp-react 主 registry 与 shopModules 店铺白名单中均无渲染入口（已核实），
 * 故 registerModule 加 visible:false 不进工具栏——仅保证存量数据可打开/预览，
 * 待渲染端补入口后再放开。契约定义与 Preview/Settings 照常做全。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultModuleStyle, defaultContentStyle } from "../../../contract/sharedDefaults";

export const storeOrderCardDefaults = {
    title: "我的订单",
    showCount: 4,
    items: [
        { title: "待付款", icon: "💰", count: 0 },
        { title: "待发货", icon: "📦", count: 0 },
        { title: "待收货", icon: "🚚", count: 0 },
        { title: "已完成", icon: "✅", count: 0 }
    ],
    backgroundColor: "#ffffff",
    titleColor: "#333333",
    itemColor: "#666666",
    badgeColor: "#e1251b",
    cardRadius: 8,
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "storeOrderCard",
    label: "店铺订单卡",
    icon: '<i class="h5-mod-ico">🧾</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 69,
    visible: false,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(storeOrderCardDefaults)
});
