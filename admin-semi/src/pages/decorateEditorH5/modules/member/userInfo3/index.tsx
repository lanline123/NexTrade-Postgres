/**
 * 会员信息(样式3) 模块（契约依据：uniapp-react pages/member/components/decorate/userInfo3/index.tsx 实际读取字段）
 * 渲染端读取：layoutStyle(1-4)、showEditIcon、signIn/settings/notification{show,icon,title}、
 * gradient{colorA,colorB,gradientType}、bgColor(旧)、backgroundImage/backgroundImageShow、
 * bottomArc{show,height}、memberBar{show,backgroundColor,backgroundColorB,borderRadius,levelName,description,buttonText,buttonLink,waveImage}、
 * stats{show,items[{key,label,show}]}、textColors{nameColor,subColor,statValueColor,statLabelColor}、frame。
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";

export const userInfo3Defaults = {
    nickname: "会员昵称",
    description: "普通会员",
    layoutStyle: 1,
    showEditIcon: 1,
    signIn: { show: 1, title: "签到有礼", icon: { picUrl: "", picThumb: "" } },
    settings: { show: 1, icon: { picUrl: "", picThumb: "" } },
    notification: { show: 1, icon: { picUrl: "", picThumb: "" } },
    gradient: { colorA: "#57D45C", colorB: "#34C759", gradientType: "leftRight" },
    bgColor: "",
    backgroundImage: { picUrl: "", picThumb: "" },
    backgroundImageShow: 1,
    bottomArc: { show: 1, height: 24 },
    memberBar: {
        show: 1,
        backgroundColor: "",
        backgroundColorB: "",
        borderRadius: 8,
        levelName: "",
        description: "",
        buttonText: "查看",
        buttonLink: { link: "", title: "" },
        waveImage: { picUrl: "", picThumb: "" }
    },
    stats: {
        show: 1,
        items: [
            { key: "collect", label: "商品收藏", show: 1 },
            { key: "follow", label: "店铺收藏", show: 1 },
            { key: "history", label: "浏览记录", show: 1 }
        ]
    },
    textColors: { nameColor: "", subColor: "", statValueColor: "", statLabelColor: "" },
    frame: { ...defaultFrame, backgroundColor: "", innerPadding: 16, boxPadding: 0, boxPaddingTop: 0, boxPaddingBottom: 0 }
};

registerModule({
    type: "userInfo3",
    label: "会员信息(样式3)",
    icon: '<i class="h5-mod-ico">👤</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 82,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(userInfo3Defaults)
});
