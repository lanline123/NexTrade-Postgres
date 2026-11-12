<template>
    <div v-if="navItems.length" class="dec-pc-home-quick-nav" :style="cssVars">
        <el-scrollbar wrap-class="dec-pc-home-quick-nav__scroll-wrap" view-class="dec-pc-home-quick-nav__scroll-view">
            <div class="nav_container fs_mod">
                <NuxtLink
                    v-for="(item, index) in navItems"
                    :key="item.picId || index"
                    class="dec-pc-home-quick-nav__item"
                    :to="getItemLink(item)"
                    target="_blank"
                    :title="item.picTitle"
                >
                    <img v-if="getItemIcon(item)" :src="getItemIcon(item)" class="dec-pc-home-quick-nav__icon" alt="" />
                    <span v-if="item.picTitle" class="dec-pc-home-quick-nav__text">{{ item.picTitle }}</span>
                </NuxtLink>
            </div>
        </el-scrollbar>
    </div>
</template>

<script lang="ts" setup>
import { computed } from "vue";
import { productImage, urlFormat } from "~/utils/format";

interface PcQuickNavItem {
    picTitle?: string;
    picThumb?: string;
    picUrl?: string;
    picLink?: string | { link?: unknown };
}

interface PcQuickNavModule {
    items?: PcQuickNavItem[];
    style?: Record<string, any>;
    frame?: Record<string, any>;
}

const props = defineProps({
    module: {
        type: Object as () => PcQuickNavModule,
        default: (): PcQuickNavModule => ({})
    }
});

const navItems = computed(() => {
    const list = props.module?.items;
    return Array.isArray(list) ? list : [];
});

const cssVars = computed(() => {
    const frame = props.module?.frame ?? {};
    const style = props.module?.style ?? {};
    const containerAlign = Number(style.containerAlign ?? 1);
    const containerJustify = containerAlign === 2 ? "center" : containerAlign === 3 ? "flex-end" : "flex-start";

    return {
        "--dec-box-padding": `${frame.boxPadding ?? 0}px`,
        "--dec-box-padding-top": `${frame.boxPaddingTop ?? 0}px`,
        "--dec-box-padding-bottom": `${frame.boxPaddingBottom ?? 0}px`,
        "--dec-box-radius": `${frame.boxRadius ?? 0}px`,
        "--dec-bg-color": frame.backgroundColor ?? "transparent",
        "--dec-font-size": `${style.fontSize ?? 14}px`,
        "--dec-text-color": style.textColor ?? "#333333",
        "--dec-hover-color": style.hoverColor ?? "#e1251b",
        "--dec-container-justify": containerJustify,
        "--dec-item-gap": `${style.itemGap ?? 16}px`,
        "--dec-single-line": Number(style.singleLine ?? 0) === 1 ? "nowrap" : "wrap",
        "--dec-font-weight": style.fontWeight ?? 400,
        "--dec-line-height": `${style.lineHeight ?? 36}px`,
        "--dec-icon-size": `${style.iconSize ?? 18}px`,
        "--dec-icon-gap": `${style.iconGap ?? 6}px`
    } as Record<string, string>;
});

const getItemIcon = (item: PcQuickNavItem): string => {
    return productImage(item);
};

const getItemLink = (item: PcQuickNavItem): string => {
    const picLink = item?.picLink;
    if (!picLink) return "";
    if (typeof picLink === "string") return urlFormat(picLink);
    if (typeof picLink === "object" && picLink !== null) {
        const fullLink = urlFormat(picLink as any);
        if (fullLink) return fullLink;
        if ("link" in picLink && picLink.link !== undefined) return urlFormat(picLink.link as any);
    }
    return "";
};
</script>

<style lang="less" scoped>
.dec-pc-home-quick-nav {
    box-sizing: border-box;
    padding-left: var(--dec-box-padding, 0);
    padding-right: var(--dec-box-padding, 0);
    padding-top: var(--dec-box-padding-top, 0);
    padding-bottom: var(--dec-box-padding-bottom, 0);
    background-color: var(--dec-bg-color, transparent);
    border-radius: var(--dec-box-radius, 0);

    :deep(.dec-pc-home-quick-nav__scroll-wrap) {
        overflow-x: auto;
        overflow-y: hidden;
    }

    :deep(.dec-pc-home-quick-nav__scroll-view) {
        display: flex;
    }

    .nav_container.fs_mod {
        display: flex;
        flex-wrap: var(--dec-single-line, wrap);
        align-items: center;
        justify-content: var(--dec-container-justify, flex-start);
        gap: var(--dec-item-gap, 16px);
        line-height: var(--dec-line-height, 36px);
        font-weight: var(--dec-font-weight, 400);
        min-width: 100%;
    }

    &__item {
        display: inline-flex;
        align-items: center;
        gap: var(--dec-icon-gap, 6px);
        font-size: var(--dec-font-size, 14px);
        font-weight: var(--dec-font-weight, 400);
        line-height: var(--dec-line-height, 36px);
        color: var(--dec-text-color, #333333);
        text-decoration: none;
        white-space: nowrap;
        transition:
            color 0.2s ease,
            transform 0.15s ease;
        cursor: pointer;

        &:hover {
            color: var(--dec-hover-color, #e1251b) !important;
        }
    }

    &__icon {
        flex-shrink: 0;
        width: var(--dec-icon-size, 18px);
        height: var(--dec-icon-size, 18px);
        object-fit: contain;
    }

    &__text {
        flex-shrink: 0;
    }
}
</style>
