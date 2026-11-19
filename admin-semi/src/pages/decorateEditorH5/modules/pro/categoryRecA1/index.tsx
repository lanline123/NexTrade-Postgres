/**
 * 分类推荐A1 模块（契约自 view/admin editor/modules/mobilePro/categoryRecA1/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultSubTitle, defaultModuleStyle, defaultContentStyle } from "../../../contract/sharedDefaults";

const defaultPicList = [
    {
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/official/img/pc/202605/1779325953rpmWhZWipxQ7nmuAQj.png",
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/official/img/pc/202605/1779325953rpmWhZWipxQ7nmuAQj.png",
        picTitle: "领取优惠",
        picDesc: "优惠券",
        picLink: { link: "", title: "" }
    },
    {
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/official/img/pc/202605/1779325953xOBpJxh9kzrNjJp3rB.png",
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/official/img/pc/202605/1779325953xOBpJxh9kzrNjJp3rB.png",
        picTitle: "爆款水果",
        picDesc: "立即探鲜",
        picLink: { link: "", title: "" }
    },
    {
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/official/img/pc/202605/1779325953xOBpJxh9kzrNjJp3rB.png",
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/official/img/pc/202605/1779325953xOBpJxh9kzrNjJp3rB.png",
        picTitle: "在家火锅",
        picDesc: "温暖冬季",
        picLink: { link: "", title: "" }
    },
    {
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/official/img/pc/202605/1779325953xOBpJxh9kzrNjJp3rB.png",
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/official/img/pc/202605/1779325953xOBpJxh9kzrNjJp3rB.png",
        picTitle: "精致生活",
        picDesc: "居家好货",
        picLink: { link: "", title: "" }
    }
];

/** 与旧 mobilePro/categoryRecA1/index.ts categoryRecA1Defaults 逐字段一致（契约，含共享常量展开覆写） */
export const categoryRecA1Defaults = {
    picList: defaultPicList,
    picFillType: "contain",
    picRadius: 8,
    subTitle: { ...cloneDeep(defaultSubTitle), titleFontSize: 14, titleFontBold: 700 },
    subTitleColor: "#999999",
    moduleStyle: { ...cloneDeep(defaultModuleStyle), boxPaddingTop: 0, boxPaddingBottom: 0, boxPadding: 6 },
    contentStyle: defaultContentStyle
};

registerModule({
    type: "categoryRecA1",
    label: "分类推荐A1",
    icon: '<i class="h5-mod-ico">🗂️</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 7,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(categoryRecA1Defaults)
});
