/**
 * 会员信息 模块（契约依据：uniapp-react pages/member/components/decorate/userInfo/index.tsx 实际读取字段）
 * 渲染端读取：signIn{show,title,icon.picUrl}、settings{show,icon.picUrl}、gradient{colorA,colorB}、frame；
 * 昵称/等级/统计数来自 user store，editor 侧用 nickname/description 演示占位。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

export const userInfoDefaults = {
    nickname: "会员昵称",
    description: "欢迎回来",
    signIn: { show: 1, title: "签到有礼", icon: { picUrl: "", picThumb: "" } },
    settings: { show: 1, icon: { picUrl: "", picThumb: "" } },
    gradient: { colorA: "#fcf6fc", colorB: "#f3f3fe" },
    frame: { ...defaultFrame, backgroundColor: "", innerPadding: 16, boxPadding: 0, boxPaddingTop: 0, boxPaddingBottom: 0 }
};

registerModule({
    type: "userInfo",
    label: "会员信息",
    icon: '<i class="h5-mod-ico">👤</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 80,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(userInfoDefaults)
});
