<template>
    <TotalContainer v-if="module.picList && module.picList.length > 0" :module="module.frame">
        <Title :module="module.title" :right-text="module.title?.moreLink?.title"></Title>
        <template v-if="module.picList">
            <div class="image-hotarea-warp" :style="format.innerPadding">
                <div class="image-ad-con">
                    <template v-for="(item, picIndex) in module.picList" :key="item.picId ?? picIndex">
                        <div class="image-ad-item">
                            <div class="item-content" :style="format.imgPadding">
                                <div :ref="(el) => setPicWrapRef(el as HTMLDivElement | null, Number(picIndex))" class="item-content-hotatrea">
                                    <div class="item-img-a">
                                        <el-image class="item-img" fit="contain" :src="imageFormat(item.picUrl)" alt="" />
                                    </div>
                                    <template v-if="item.hotarea">
                                        <div
                                            v-for="(subItem, index) in item.hotarea"
                                            :key="index"
                                            class="image-hotarea-link"
                                            :data-width="subItem.width"
                                            :data-height="subItem.height"
                                            :data-left="subItem.pageX ?? subItem.left"
                                            :data-top="subItem.pageY ?? subItem.top"
                                            :style="getHotareaStyle(subItem, Number(picIndex))"
                                            @click="subItem.link && handleClick(subItem.link)"
                                        >
                                            <span class="area_box_con_text">{{ subItem.link?.name }}</span>
                                        </div>
                                    </template>
                                </div>
                            </div>
                        </div>
                    </template>
                </div>
            </div>
        </template>
    </TotalContainer>
</template>

<script lang="ts" setup>
import { computed, ref, onUnmounted } from "vue";
import TotalContainer from "~/decorate/components/shop/TotalContainer.vue";
import Title from "~/decorate/components/shop/Title.vue";
import { imageFormat, urlFormat } from "~/utils/format";

const props = defineProps({
    module: {
        type: Object,
        default: () => ({})
    }
});

// 热区绘制区域原始宽度（与后台预览基准一致）
const HOTAREA_ORIGINAL_WIDTH = 500;

// 每张图片容器宽度（用于把 500px 基准坐标缩放到实际渲染宽度）
const picWrapWidths = ref<Record<number, number>>({});
const picWrapEls = new Map<number, HTMLDivElement>();
let resizeObserver: ResizeObserver | null = null;

const format = computed(() => {
    // 左右内边距对齐后台「容器设置」：优先 leftAndRightPadding，回退 innerPadding
    const innerPad = props.module.frame?.leftAndRightPadding ?? props.module.frame?.innerPadding ?? 0;
    return {
        imgPadding: "padding:" + (props.module?.imgPadding || 0) + "px;",
        innerPadding: innerPad ? "padding:" + innerPad + "px;" : ""
    };
});

function toNumber(val: unknown): number {
    const n = Number(val);
    return Number.isFinite(n) ? n : 0;
}

interface HotareaBox {
    width?: number | string;
    height?: number | string;
    left?: number | string;
    top?: number | string;
    pageX?: number | string;
    pageY?: number | string;
    position?: string;
}

function resizeValue(size: number, picIndex: number): number {
    const width = picWrapWidths.value[picIndex] || 0;
    if (!width) return size;
    return (width / HOTAREA_ORIGINAL_WIDTH) * size;
}

function getHotareaStyle(hotarea: HotareaBox | null | undefined, picIndex: number): Record<string, string> | string {
    // 兜底：没测到宽度（SSR 或 ResizeObserver 不可用）时，优先用后端下发的 position 字符串
    const width = picWrapWidths.value[picIndex] || 0;
    if (!width) return hotarea?.position || "";

    const w = toNumber(hotarea?.width);
    const h = toNumber(hotarea?.height);
    const left = toNumber(hotarea?.left ?? hotarea?.pageX);
    const top = toNumber(hotarea?.top ?? hotarea?.pageY);

    return {
        width: `${resizeValue(w, picIndex)}px`,
        height: `${resizeValue(h, picIndex)}px`,
        left: `${resizeValue(left, picIndex)}px`,
        top: `${resizeValue(top, picIndex)}px`
    };
}

function ensureResizeObserver(): void {
    if (resizeObserver) return;
    if (typeof window === "undefined" || typeof ResizeObserver === "undefined") return;
    resizeObserver = new ResizeObserver((entries: ResizeObserverEntry[]) => {
        let changed = false;
        const next: Record<number, number> = { ...picWrapWidths.value };
        for (const entry of entries) {
            const target = entry.target as HTMLDivElement;
            const idx = Number(target.dataset.hotareaIndex ?? -1);
            if (idx >= 0) {
                const width = Number(entry.contentRect.width.toFixed(2));
                if (next[idx] !== width) {
                    next[idx] = width;
                    changed = true;
                }
            }
        }
        if (changed) {
            picWrapWidths.value = next;
        }
    });
}

function setPicWrapRef(el: HTMLDivElement | null, index: number): void {
    ensureResizeObserver();
    const prev = picWrapEls.get(index);
    if (!el) {
        if (prev && resizeObserver) resizeObserver.unobserve(prev);
        picWrapEls.delete(index);
        if (Object.prototype.hasOwnProperty.call(picWrapWidths.value, index)) {
            const { [index]: _, ...rest } = picWrapWidths.value;
            picWrapWidths.value = rest;
        }
        return;
    }

    // ref 回调每次渲染都会执行，避免无意义写入导致递归更新
    if (prev === el) {
        const width = Number(el.offsetWidth.toFixed(2));
        if (picWrapWidths.value[index] !== width) {
            picWrapWidths.value = { ...picWrapWidths.value, [index]: width };
        }
        return;
    }

    if (prev && resizeObserver) resizeObserver.unobserve(prev);
    el.dataset.hotareaIndex = String(index);
    picWrapEls.set(index, el);
    const width = Number(el.offsetWidth.toFixed(2));
    if (picWrapWidths.value[index] !== width) {
        picWrapWidths.value = { ...picWrapWidths.value, [index]: width };
    }
    if (resizeObserver) resizeObserver.observe(el);
}

const handleClick = (link: any): void => {
    if (!link) return;
    const url = urlFormat(link);
    if (url) {
        if (url.startsWith("http")) {
            window.open(url, "_blank");
        } else {
            navigateTo(url);
        }
    }
};

onUnmounted(() => {
    if (resizeObserver) {
        resizeObserver.disconnect();
        resizeObserver = null;
    }
    picWrapEls.clear();
});
</script>

<style lang="less" scoped>
.image-hotarea-warp {
    box-sizing: border-box;
    width: 100%;

    .image-ad-con {
        width: 100%;
    }

    .image-ad-item {
        width: 100%;
        position: relative;
        margin-bottom: 10px;

        &:last-child {
            margin-bottom: 0;
        }

        .item-content {
            width: 100%;
            box-sizing: border-box;
            position: relative;

            .item-content-hotatrea {
                position: relative;
                width: 100%;
                display: flex;

                .item-img-a {
                    width: 100%;
                    position: relative;

                    .item-img {
                        width: 100%;
                        height: auto;
                        display: block;
                    }
                }

                .image-hotarea-link {
                    position: absolute;
                    cursor: pointer;
                    z-index: 10;
                    transition: opacity 0.3s;

                    &:hover {
                        opacity: 0.8;
                    }

                    .area_box_con_text {
                        display: none;
                    }
                }
            }
        }
    }
}
</style>
