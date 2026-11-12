<template>
    <div
        :style="{
            background: module.titleBackground2
                ? 'linear-gradient(to right, ' + module.titleBackground + ', ' + module.titleBackground2 + ')'
                : module.titleBackground,
            'border-top-left-radius': module.titleRadius + 'px',
            'border-top-right-radius': module.titleRadius + 'px'
        }"
        v-if="module.showTitle == 1"
        class="top-bar"
    >
        <div class="left-title">
            <div class="line" v-if="module.titleStyle == 3"></div>
            <el-image v-if="module.titleBackgroundPic?.picUrl" class="image-size" :src="imageFormat(module.titleBackgroundPic.picUrl)"></el-image>
            <div :style="{ color: module.titleColor }" class="big">{{ module.titleText }}</div>
            <div :style="{ color: module.descColor }" :class="[module.titleStyle == 2 ? 'big-sm' : 'sm']">
                {{ module.descText }}
            </div>
        </div>
        <NuxtLink target="_blank" :to="urlFormat(module.moreLink)" v-if="module.showMore == 1" :style="{ color: module.moreColor }" class="right-title">
            {{ $t(rightText) }}
        </NuxtLink>
    </div>
</template>
<script setup lang="ts">
import { defineProps, ref } from "vue";
import { imageFormat, urlFormat } from "~/utils/format";
const props = defineProps({
    module: { type: Object, default: {} },
    rightText: { type: String, default: "查看更多" }
});
const rightText = ref(props.rightText);
</script>
<style scoped lang="less">
.top-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-direction: row;
    width: 100%;
    padding: 5px 10px 3px;
    box-sizing: border-box;

    .left-title {
        display: flex;
        align-items: center;
        gap: 4px;
        .line {
            width: 4px;
            height: 16px;
            background-color: white;
            border-radius: 8px;
        }

        .image-size {
            width: 30px;
            height: 30px;
            margin-left: 10px;
        }

        .big {
            font-size: 16px;
            margin-right: 5px;
            font-weight: bold;
        }

        .sm {
            font-size: 12px;
        }
        .big-sm {
            font-size: 16px;
        }
        .special {
        }
    }

    .right-title {
        cursor: pointer;
    }
}
</style>
