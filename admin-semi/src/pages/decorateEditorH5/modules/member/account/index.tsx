/**
 * 账户信息 模块（契约依据：uniapp-react pages/member/components/decorate/account/index.tsx 实际读取字段）
 * 渲染端读取：moduleTitle、accountList[{picUrl,picThumb,picTitle,picDesc,picLink.link,isShow}]、frame；
 * accountList 为空时渲染端用内置默认四项（账号详情/国家地区和语言/地址/设置），此处默认值与之同题。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

const acc = (picTitle: string) => ({ picUrl: "", picThumb: "", picTitle, picDesc: "", picLink: { link: "", title: "" }, isShow: 1 });

export const accountDefaults = {
    moduleTitle: "账号",
    accountList: [acc("账号详情"), acc("国家/地区和语言"), acc("地址"), acc("设置")],
    frame: { ...defaultFrame, backgroundColor: "#ffffff", boxRadius: 0, innerPadding: 15, boxPadding: 0, boxPaddingTop: 10, boxPaddingBottom: 0 }
};

registerModule({
    type: "account",
    label: "账户信息",
    icon: '<i class="h5-mod-ico">💳</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 89,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(accountDefaults)
});
