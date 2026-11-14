/**
 * 会员信息(样式2) 模块（契约依据：uniapp-react pages/member/components/decorate/userInfoV2/index.tsx）
 * 渲染端仅读取 frame（头像/昵称/等级均来自 user store）；nickname/description 为 editor 演示占位。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

export const userInfoV2Defaults = {
    nickname: "会员昵称",
    description: "为了获得最佳体验",
    frame: { ...defaultFrame, backgroundColor: "#ffffff", innerPadding: 16, boxPadding: 0, boxPaddingTop: 0, boxPaddingBottom: 0 }
};

registerModule({
    type: "userInfoV2",
    label: "会员信息(样式2)",
    icon: '<i class="h5-mod-ico">👤</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 81,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(userInfoV2Defaults)
});
