/**
 * 商品详情页「规格图 → 主图区」联动
 *
 * 规格区与主图区是并列的兄弟组件，由商详页统一提供状态：
 * - 规格区在用户主动切换规格时写入命中的规格图
 * - 主图区把规格图叠加在图集浏览态之上，不改动 picList 本身
 */
import type { ComputedRef, InjectionKey, Ref } from "vue";
import { imageFormat } from "@/utils/format";

interface SpecPicContext {
    /** 当前规格图地址，空串表示回到商品主图 */
    picUrl: Ref<string>;
    /** 每次用户主动切换规格自增，用于识别「选中另一个同图规格」这类地址未变的切换 */
    version: Ref<number>;
}

interface SpecPicGalleryOptions {
    /** 主图区当前的图集（含视频项） */
    list: ComputedRef<any[]> | Ref<any[]>;
    /** 图集浏览态的当前下标 */
    imgIndex: Ref<number>;
    /** 规格图生效时的附加处理，如退出视频态 */
    onApply?: () => void;
}

const specPicKey = Symbol("productSpecPic") as InjectionKey<SpecPicContext>;

/** 商详页：下发规格图状态 */
export const useProvideSpecPic = () => {
    const context: SpecPicContext = {
        picUrl: ref(""),
        version: ref(0)
    };
    provide(specPicKey, context);
    return context;
};

/**
 * 规格区：写入用户主动选中的规格图
 * 传空值表示所选规格都没有配图，主图区回到商品主图
 */
export const useSpecPicWriter = () => {
    const context = inject(specPicKey, null);
    return (picUrl?: string) => {
        if (!context) return;
        context.picUrl.value = picUrl || "";
        context.version.value += 1;
    };
};

/** 主图区：消费规格图，返回主图显示区应该用的图与缩略图高亮判断 */
export const useSpecPicGallery = (options: SpecPicGalleryOptions) => {
    const context = inject(specPicKey, null);
    const { list, imgIndex } = options;

    // 规格图不在 picList 中时的独立展示态；命中 picList 时走图集自然定位，此值为空
    const standalonePic = ref("");

    // 图集中第一张商品图的下标（视频占首位时为 1）
    const firstPicIndex = () => list.value.findIndex((item) => !item?.videoUrl);

    // 图集内的图片地址常带 oss 处理参数，比较前统一补全前缀并去掉参数
    const normalizePic = (picUrl?: string) => (picUrl ? imageFormat(picUrl).split("?")[0] : "");

    // 规格图在图集中的下标，未命中返回 -1
    const findPicIndex = (picUrl: string) => {
        const target = normalizePic(picUrl);
        if (!target) return -1;
        return list.value.findIndex((item) => {
            if (!item || item.videoUrl) return false;
            return [item.picUrl, item.picLarge, item.picThumb].some((pic) => pic && normalizePic(pic) === target);
        });
    };

    /** 主图显示区与放大镜共用的图：规格图优先，否则走图集当前项 */
    const activeImage = computed(() => {
        if (standalonePic.value) {
            return { picUrl: standalonePic.value, picLarge: standalonePic.value };
        }
        return list.value[imgIndex.value];
    });

    /** 缩略图高亮：处于独立规格图态时不高亮任何一项 */
    const isThumbActive = (index: number) => !standalonePic.value && imgIndex.value === index;

    /** 用户浏览图集（点缩略图 / 左右箭头）时退出规格图态 */
    const exitSpecPic = () => {
        standalonePic.value = "";
    };

    if (context) {
        watch(context.version, () => {
            const picUrl = context.picUrl.value;
            const matchedIndex = picUrl ? findPicIndex(picUrl) : firstPicIndex();
            if (matchedIndex > -1) {
                standalonePic.value = "";
                imgIndex.value = matchedIndex;
            } else {
                standalonePic.value = picUrl;
            }
            options.onApply?.();
        });
    }

    // 商品数据重新加载后图集已换，残留的规格图态需要清掉
    watch(list, exitSpecPic);

    return { activeImage, isThumbActive, exitSpecPic };
};
