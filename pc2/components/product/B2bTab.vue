<template>
    <!-- 跨境版 UI：下划线菜单风格 -->
    <div class="b2b-tab-overseas" v-if="isOverseasUi">
        <div class="tab-main">
            <div v-for="item in overseasMenuList" :class="tabIndex == item.type ? 'active' : ''" class="tab-cell" @click="handleTabChange(item.type)">
                <div :class="tabIndex == item.type ? 'fouerty' : ''" class="font-style">
                    {{ $t(item.label) }}
                    <div class="line"></div>
                </div>
            </div>
        </div>
    </div>
    <div class="tab" :class="{ 'new-ui': uiVersion === 'new' }" v-else>
        <div class="tab-item" :class="{ active: tabIndex === 0 }" @click="handleTabChange(0)">
            {{ $t("单品采购") }}
            <img class="image" style="right: 0" v-show="tabIndex === 1" src="@/assets/images/product/shadow-right.png" />
        </div>
        <div class="tab-item" :class="{ active: tabIndex === 1 }" @click="handleTabChange(1)">
            {{ $t("批量采购") }}
            <img class="image" style="left: 0" v-show="tabIndex === 0" src="@/assets/images/product/shadow-left.png" />
        </div>
        <div class="line" :style="`${tabIndex === 0 ? 'left: 0' : 'right: 0'}`"></div>
        <div class="subline" :style="`${tabIndex === 0 ? 'right: 0' : 'left: 0'}`"></div>
    </div>
</template>

<script setup lang="ts">
import { computed, unref, inject } from "vue";
defineProps({
    // UI 版本：old 老版，new 新版（新版为分段控件风格）
    uiVersion: {
        type: String,
        default: "old"
    }
});
// 跨境版 UI 开关（由商详页 Product.vue provide 下发；非跨境时默认原逻辑）
const detailUiVersion = inject("detailUiVersion", null);
const isOverseasUi = computed(() => unref(detailUiVersion) === "overseas");
const overseasMenuList = [
    { type: 0, label: "单品采购" },
    { type: 1, label: "批量采购" }
];
const tabIndex = defineModel("tabIndex", { type: Number });
const handleTabChange = (index: number) => {
    tabIndex.value = index;
};
</script>

<style lang="less" scoped>
.tab {
    width: 100%;
    display: flex;
    position: relative;
    margin-top: 10px;
    cursor: pointer;
    .line,
    .subline {
        position: absolute;
        width: 50%;
        height: 2px;
        background-color: var(--general);
    }
    .line {
        top: 0;
    }
    .subline {
        bottom: 0;
        height: 1px;
        background-color: #eee;
    }

    .tab-item {
        width: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        height: 42px;
        font-size: 14px;
        font-weight: 500;
        position: relative;

        .image {
            width: 10px;
            height: 100%;
            position: absolute;
            top: 0;
        }
        &.active {
            color: var(--general);
        }
    }

    // 新版 UI：分段控件（segmented control）风格
    &.new-ui {
        box-sizing: border-box;
        margin-top: 0;
        margin-bottom: 24px;
        padding: 4px;
        // max-width: 490px;
        background: #f5f5f7;
        border-radius: 12px;
        font-family: Inter, "SF Pro Text", Roboto, "Helvetica Neue", Helvetica, Tahoma, Arial, "PingFang SC", "Microsoft YaHei";

        .line,
        .subline {
            display: none;
        }

        .tab-item {
            height: 36px;
            border-radius: 8px;
            color: #999;
            font-size: 14px;
            font-weight: 500;
            transition: background-color 0.2s ease, color 0.2s ease, box-shadow 0.2s ease;

            .image {
                display: none;
            }

            &.active {
                background: #fff;
                color: rgba(9, 9, 10, 0.92);
                box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
            }
        }
    }
}

// 跨境版 B2B tab 样式，独立根类名隔离
.b2b-tab-overseas {
    .tab-main {
        position: relative;
        display: flex;
        background: #fff;
        color: #000;
        border-bottom: 1px solid #c1c1c1;
        margin-bottom: 10px;
        .active {
            color: #000;
        }
        .fouerty {
            position: relative;
            .line {
                position: absolute;
                bottom: -2px;
                width: 100%;
                height: 4px;
                background-color: #000;
                border-radius: 10px;
            }
        }
        .font-style {
            padding: 10px 0px 12px 0;
            margin-right: 30px;
            font-size: 16px;
            font-weight: 600;
            cursor: pointer;
        }
    }
}
</style>
