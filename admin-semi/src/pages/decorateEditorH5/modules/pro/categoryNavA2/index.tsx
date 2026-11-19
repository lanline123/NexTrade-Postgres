/**
 * 类目导航A2 模块（契约自 view/admin editor/modules/mobilePro/categoryNavA2/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultAnimation, defaultSubTitle, defaultModuleStyle, defaultContentStyle } from "../../../contract/sharedDefaults";

const categoryList = [
    {
        picId: 1390,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958340xhqljwDakcNeJXCPna.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958340xhqljwDakcNeJXCPna.png",
        picName: "FoAbizq7oeHiwJzoJVaBmgV50oRN",
        picTitle: "折扣专区"
    },
    {
        picId: 1396,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958341AwZmIgqi2MgyFeZuUt.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958341AwZmIgqi2MgyFeZuUt.png",
        picName: "Ft0N3qmFphVAkmv8Pn6QDp61Qw7",
        picTitle: "运动旅行"
    },
    {
        picId: 1398,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958342pNWuK2IUhmpjICKmzC.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958342pNWuK2IUhmpjICKmzC.png",
        picName: "FuiW1ysk28blbkbFpw6FHFJDKiOA",
        picTitle: "个护护理"
    },
    {
        picId: 1393,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17309583411C1elxz6qd8TAbdnTB.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/17309583411C1elxz6qd8TAbdnTB.png",
        picName: "Fra7mrH3EH2NejP7y3K9LrcoGEaH",
        picTitle: "彩妆护肤"
    },
    {
        picId: 1392,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958341pqcAwTAnyf1I8qQtRN.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958341pqcAwTAnyf1I8qQtRN.png",
        picName: "FrPMN2wDsjJoc6ydlCHge82oon6R",
        picTitle: "母婴用品"
    },
    {
        picId: 1387,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958340219IHpDSXO5TlSVDNb.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958340219IHpDSXO5TlSVDNb.png",
        picName: "FjF9irxfLxeOpMunPtgK2Gjoocb",
        picTitle: "数码家电"
    },
    {
        picId: 1395,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958341ltUrGddGs3uvLccUTu.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958341ltUrGddGs3uvLccUTu.png",
        picName: "Fr9nt9qxQxefziAVXZIGkQZx1goQ",
        picTitle: "洗护用品"
    },
    {
        picId: 1399,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958342xoZNYWFWnp2ltRzpzO.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958342xoZNYWFWnp2ltRzpzO.png",
        picName: "FulNvSiPYQdW_EOcqb851h3fuIrp",
        picTitle: "整理收纳"
    },
    {
        picId: 1386,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958340uaWpBXrJkO4hghTti6.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958340uaWpBXrJkO4hghTti6.png",
        picName: "FnbMGPun7yw92rulVAnOChQpfukU",
        picTitle: "家居生活"
    },
    {
        picId: 1397,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958341CjpCfA4M71ErjOZuBA.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958341CjpCfA4M71ErjOZuBA.png",
        picName: "Ftn_vGbQk_MQfqgB0ttsh0dWhj",
        picTitle: "服装服饰"
    },
    {
        picId: 1394,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958341DTU3wlQ5Iv2Phj2RUd.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958341DTU3wlQ5Iv2Phj2RUd.png",
        picName: "Fs4k1iKK6V1vSz1BeXlrHmMPCNwZ",
        picTitle: "家居生活"
    },
    {
        picId: 1389,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958340zy5c1MTCtT41yA6Y7X.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958340zy5c1MTCtT41yA6Y7X.png",
        picName: "FkmgdX63YoSx3nXZLroarmC5b",
        picTitle: "美味零食"
    },
    {
        picId: 1391,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958340LIruk4KbAEdw3LgOA6.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958340LIruk4KbAEdw3LgOA6.png",
        picName: "FpttgR5n4ZIHVXn4jf0_ZM9PpBl",
        picTitle: "酒水饮料"
    },
    {
        picId: 1388,
        picThumb: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958340Jh4hoAMEWqjuMWVSzf.png?x-oss-process=image/resize,m_pad,h_200,h_200",
        picUrl: "https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202411/1730958340Jh4hoAMEWqjuMWVSzf.png",
        picName: "Fmv5JqMd6qokW4bBjoSaBTWEaMt",
        picTitle: "鲜花礼品"
    }
];
;

/** 与旧 mobilePro/categoryNavA2/index.ts categoryNavA2Defaults 逐字段一致（契约） */
export const categoryNavA2Defaults = {
    listStyleType: "list",
    categoryList,
    rowNum: "custom",
    rowPicPercent: 18,
    animation: defaultAnimation,
    picFillType: "cover",
    picPercent: 90,
    radius: 200,
    categoryPadding: 10,
    subTitle: defaultSubTitle,
    indicatorBar: 1,
    barWidth: 30,
    barHeight: 2,
    barWidthOn: 30,
    barColor: "#eeeeee",
    barColorOn: "#ff6600",
    moduleStyle: defaultModuleStyle,
    contentStyle: defaultContentStyle
};

registerModule({
    type: "categoryNavA2",
    label: "类目导航A2",
    icon: '<i class="h5-mod-ico">🧭</i>',
    url: "mobilePro",
    platform: "mobile",
    order: 20,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(categoryNavA2Defaults)
});
