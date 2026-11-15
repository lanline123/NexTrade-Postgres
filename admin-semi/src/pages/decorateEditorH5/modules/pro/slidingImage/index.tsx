/**
 * 小图滑动 模块（契约自 view/admin editor/modules/mobilePro/slidingImage/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultSubTitle, defaultModuleStyle } from "../../../contract/sharedDefaults";

const picList = [
    {
        picId: 1417,
        picThumb: "https://oss.tigshop.com/img/gallery/202411/1731293094u3csMxzo4Wzksk1uTw.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://oss.tigshop.com/img/gallery/202411/1731293094u3csMxzo4Wzksk1uTw.jpg",
        picName: "FvsG9vzOZj0aUGNdgE0VrqzDqpu",
        gradientColorA: "rgba(235, 152, 115, 1)",
        gradientColorB: "rgba(234, 92, 88, 1)",
        picTitle: "每日特价",
        picDesc: "2件5折"
    },
    {
        picId: 1418,
        picThumb: "https://oss.tigshop.com/img/gallery/202411/17312930944dZgpDLjByAzYyejYt.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://oss.tigshop.com/img/gallery/202411/17312930944dZgpDLjByAzYyejYt.jpg",
        picName: "Fru1GfeNP4JIvRLqI61o8NT36ZIe",
        gradientColorA: "rgba(243, 115, 130, 1)",
        gradientColorB: "rgba(229, 65, 116, 1)",
        picTitle: "精选好货",
        picDesc: "9.9秒杀"
    },
    {
        picId: 1420,
        picThumb: "https://oss.tigshop.com/img/gallery/202411/1731293094BFUdJ8FAzQ6sHI9cs8.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://oss.tigshop.com/img/gallery/202411/1731293094BFUdJ8FAzQ6sHI9cs8.jpg",
        picName: "Fmkw5_XrRUmaNYs1jk2858je58Qi",
        gradientColorA: "rgba(219, 123, 248, 1)",
        gradientColorB: "rgba(154, 39, 241, 1)",
        picTitle: "加量不加价",
        picDesc: "最高加一倍"
    },
    {
        picId: 1419,
        picThumb: "https://oss.tigshop.com/img/gallery/202411/1731293094bHw28wIxfKlXorX1Rs.jpg?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://oss.tigshop.com/img/gallery/202411/1731293094bHw28wIxfKlXorX1Rs.jpg",
        picName: "FglfJu8K11jusC9gUKjkZU3mZ3eE",
        gradientColorA: "rgba(116, 214, 172, 1)",
        gradientColorB: "rgba(98, 193, 96, 1)",
        picTitle: "下单即发货",
        picDesc: "满49包邮"
    }
];

/** 与旧 mobilePro/slidingImage/index.ts slidingImageDefaults 逐字段一致（契约） */
export const slidingImageDefaults = {
    picList,
    backgroundGradientType: "diagonal",
    picFillType: "cover",
    picWidth: 25,
    picMargin: 0,
    picPadding: 4,
    picRadius: 15,
    picReducePercent: 0.85,
    subTitle: defaultSubTitle,
    showDesc: 1,
    descColor: "#ffffff",
    moduleStyle: defaultModuleStyle,
    autoPlay: 1,
    interval: 5
};

registerModule({
    type: "slidingImage",
    label: "小图滑动",
    icon: '<i class="h5-mod-ico">🎞️</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 15,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(slidingImageDefaults)
});
