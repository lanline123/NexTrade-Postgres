/**
 * 我的服务 模块（契约依据：uniapp-react member decorate 分发——type=myService 实际渲染 MyServiceV2）
 * 渲染端读取：moduleTitle、layoutStyle(1 列表/2 宫格)、iconRadius(%)、serviceList[{picUrl,picThumb,picTitle,picLink}]、frame。
 * 注：历史 menus 协议走另一兜底组件（menuType/menuList），不在本模块契约内。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

const svc = (picTitle: string) => ({ picUrl: "", picThumb: "", picTitle, picLink: { link: "", title: "" } });

export const myServiceDefaults = {
    moduleTitle: "我的服务",
    layoutStyle: 1,
    iconRadius: 0,
    serviceList: [svc("地址管理"), svc("我的收藏"), svc("发票管理"), svc("联系客服")],
    frame: { ...defaultFrame, backgroundColor: "#ffffff", boxRadius: 10, innerPadding: 15, boxPadding: 10, boxPaddingTop: 10, boxPaddingBottom: 0 }
};

registerModule({
    type: "myService",
    label: "我的服务",
    icon: '<i class="h5-mod-ico">🛎️</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 88,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(myServiceDefaults)
});
