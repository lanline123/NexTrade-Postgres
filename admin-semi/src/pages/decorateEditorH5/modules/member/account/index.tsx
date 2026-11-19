/**
 * 账号 模块（契约自 view/admin editor/modules/mobileMember/account/index.ts 逐字段移植）
 */
import { registerModule } from "../../../contract/moduleRegistry";
import { cloneDeep, defaultFrame } from "../../../contract/sharedDefaults";
import { isOverseas } from "../../../contract/version";

const PIC = "?x-oss-process=image/resize,m_pad,h_200,h_200";
const pic = (name: string) => `https://nlatstore.cn-qingdao.oss.aliyuncs.com/img/gallery/202605/${name}${PIC}`;

/** 与旧 mobileMember/account/View.vue 的 defaultModule 逐字段一致（契约）；「国家/地区和语言」项 isShow 在创建时按 isOverseas() 求值（同原版） */
export function makeAccountDefaults() {
    return {
        moduleTitle: "账号",
        accountList: [
            { picUrl: pic("1779691120KR8ldT5BHs0ZKZ4Uxk.jpeg"), picThumb: pic("1779691120KR8ldT5BHs0ZKZ4Uxk.jpeg"), picTitle: "账号详情", picDesc: "" },
            {
                picUrl: pic("1779852393ma6VwQR9GVkKChy1d6.jpeg"),
                picThumb: pic("1779852393ma6VwQR9GVkKChy1d6.jpeg"),
                picTitle: "国家/地区和语言",
                picDesc: "",
                isShow: isOverseas()
            },
            { picUrl: pic("1779691120QiTfocEWPiXAr7ILcG.jpeg"), picThumb: pic("1779691120QiTfocEWPiXAr7ILcG.jpeg"), picTitle: "地址", picDesc: "" },
            { picUrl: pic("1779691120penAIAi6kRrFMWeeoe.jpeg"), picThumb: pic("1779691120penAIAi6kRrFMWeeoe.jpeg"), picTitle: "设置", picDesc: "" }
        ],
        frame: {
            ...cloneDeep(defaultFrame),
            backgroundColor: "#ffffff",
            boxRadius: 0,
            innerPadding: 15,
            boxPadding: 0,
            boxPaddingTop: 10,
            boxPaddingBottom: 0
        }
    };
}

export const accountDefaults = makeAccountDefaults();

registerModule({
    type: "account",
    label: "账号",
    icon: '<i class="h5-mod-ico">💳</i>',
    url: "mobileMember",
    platform: "mobile",
    order: 10,
    preview: () => import("./Preview"),
    settings: () => import("./Settings"),
    getDefaultValue: () => cloneDeep(makeAccountDefaults())
});
