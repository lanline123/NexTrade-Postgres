<template>
    <el-popover
        v-if="imageSearchStore.isAvailable(scopedShopId !== undefined)"
        :visible="visible"
        :placement="placement"
        :width="237"
        :offset="12"
        :popper-options="popperOptions"
        :persistent="false"
        :popper-style="themeStore.themeColor"
        popper-class="image-search-popper"
    >
        <template #reference>
            <span ref="triggerRef" class="image-search-entry">
                <slot :toggle="toggle" :visible="visible">
                    <el-tooltip
                        :content="$t('上传和粘贴图片搜同款')"
                        placement="bottom"
                        effect="dark"
                        :offset="10"
                        :disabled="visible"
                        :show-after="100"
                        popper-class="image-search-tip"
                    >
                        <button type="button" class="image-search-entry__btn" :aria-label="$t('以图搜图')" :aria-expanded="visible" @click.stop="toggle">
                            <CameraIcon />
                        </button>
                    </el-tooltip>
                </slot>
            </span>
        </template>
        <ImageSearchPanel v-if="visible" @close="close" @submit="onSubmit" />
    </el-popover>
</template>

<script setup lang="ts">
/**
 * 以图搜图入口：相机按钮 + 上传弹层
 * 本端未开启或开关读取失败时不渲染；默认插槽可替换触发按钮
 * 传入店铺 ID 时只搜该店，还要看店铺开关
 */
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import CameraIcon from "~/components/icons/CameraIcon.vue";
import ImageSearchPanel from "./ImageSearchPanel.vue";
import { useImageSearchStore } from "~/store/imageSearch";
import { useThemeStore } from "~/store/theme";

const props = withDefaults(
    defineProps<{
        placement?: "bottom-end" | "bottom-start" | "bottom";
        /** 店铺内搜索时传店铺 ID，0 为平台自营；不传为全站 */
        shopId?: number | string;
    }>(),
    {
        placement: "bottom",
        shopId: undefined
    }
);

const scopedShopId = computed(() => {
    const shopId = Number(props.shopId);
    return props.shopId !== undefined && props.shopId !== "" && Number.isInteger(shopId) && shopId >= 0 ? shopId : undefined;
});

const route = useRoute();
const imageSearchStore = useImageSearchStore();
// 弹层挂在 body 下，需要带上主题色变量
const themeStore = useThemeStore();

const RESULT_PATH = "/search/image";

// 弹层固定在入口下方，窗口变窄时不平移、不翻转
const popperOptions = {
    modifiers: [
        { name: "preventOverflow", enabled: false },
        { name: "flip", enabled: false }
    ]
};

const visible = ref(false);
const triggerRef = ref<HTMLElement | null>(null);

const open = (): void => {
    visible.value = true;
};

const close = (): void => {
    if (!visible.value) return;
    visible.value = false;
    // 焦点回到入口
    const target = triggerRef.value?.querySelector<HTMLElement>("button, [tabindex]");
    target?.focus();
};

const toggle = (): void => {
    if (visible.value) {
        close();
    } else {
        open();
    }
};

/** 图片已选定：交给结果页上传检索，地址上暂时没有图搜编号 */
const onSubmit = async (payload: { file: File }): Promise<void> => {
    const previewId = Number(route.query.previewId) > 0 ? String(route.query.previewId) : "";
    const shopId = scopedShopId.value !== undefined ? String(scopedShopId.value) : "";
    visible.value = false;
    imageSearchStore.setPending(payload.file);
    await navigateTo({ path: RESULT_PATH, query: { ...(shopId ? { shopId } : {}), ...(previewId ? { previewId } : {}) } });
};

/** 点击弹层和入口以外的区域关闭 */
const onDocumentPointerDown = (event: MouseEvent): void => {
    const target = event.target as Element | null;
    if (!target) return;
    if (triggerRef.value?.contains(target) || target.closest(".image-search-popper")) return;
    close();
};

watch(visible, (value) => {
    if (value) {
        document.addEventListener("mousedown", onDocumentPointerDown);
    } else {
        document.removeEventListener("mousedown", onDocumentPointerDown);
    }
});

onMounted(() => {
    void imageSearchStore.fetchCapabilities();
});

onBeforeUnmount(() => {
    document.removeEventListener("mousedown", onDocumentPointerDown);
});

defineExpose({ open, close });
</script>

<style scoped lang="less">
.image-search-entry {
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
    // 右侧紧挨搜索按钮，多留一些间距
    margin: 0 12px 0 4px;

    &__btn {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 32px;
        height: 32px;
        padding: 0;
        border: 0;
        border-radius: 4px;
        background: transparent;
        color: #8c8c8c;
        cursor: pointer;
        transition: color 0.2s;

        &:focus-visible {
            color: var(--general);
            outline: 1px solid var(--general);
            outline-offset: -1px;
        }
    }
}
</style>

<style lang="less">
.image-search-tip.el-popper.is-dark {
    padding: 8px 16px;
    border: 0;
    border-radius: 8px;
    background: #000;
    color: #fff;
    font-size: 14px;
    line-height: 20px;

    .el-popper__arrow::before {
        border: 0;
        background: #000;
    }
}

.image-search-popper.el-popover.el-popper {
    padding: 0;
    border: 0;
    min-width: 0;
    border-radius: 8px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.14);
    z-index: 9000 !important;

    // 指向入口的三角：36 × 15，下半部分压在弹层内，露出约 8px
    .el-popper__arrow {
        width: 36px;
        height: 15px;
        z-index: 0;

        &::before {
            width: 36px;
            height: 15px;
            border: 0 !important;
            border-radius: 0;
            background: #fff !important;
            transform: none !important;
            clip-path: polygon(50% 0, 100% 100%, 0 100%);
        }
    }

    &[data-popper-placement^="bottom"] > .el-popper__arrow {
        top: -8px;
    }
}
</style>
