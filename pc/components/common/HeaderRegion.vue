<template>
    <div class="region-select">
        <div class="item" :class="{ hover: showMinArea === true }" @mouseenter="showMinArea = true" @mouseleave="showMinArea = false">
            <span class="iconfont-pc icon-dizhi ico-color"></span>
            <div
                :style="{ color: colorType == 3 ? (showMinArea ? 'var(--general)' : fontColor) : showMinArea ? 'var(--general)' : fontColor }"
                class="cityname"
            >
                {{ regionname }}
            </div>
        </div>
        <div class="dropdown-mini-area" v-show="showMinArea" @mouseenter="showMinArea = true" @mouseleave="showMinArea = false">
            <div class="areamini-content-wrap">
                <div class="areamini-content-list">
                    <div class="cityitem" v-for="(item, index) in commonStore.topRegion" :key="index">
                        <a :class="{ selected: userProvinceId === item.regionId }" @click="selectRegion(item.regionId)">{{ item.regionName }}</a>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref, onMounted } from "vue";
import { getTopRegion, getUserRegion } from "@/api/common";
import { useCommonStore } from "@/store/common";
const commonStore = useCommonStore();
const props = defineProps({
    pageType: {
        type: Number,
        default: 1
    },
    colorType: {
        type: Number,
        default: 1
    }
});
const fontColor = computed(() => {
    let temp = "";
    if (props.pageType == 1) {
        temp = "black";
    } else {
        if (props.colorType == 1 || props.colorType == 2) {
            temp = "#b0b0b0";
        } else if (props.colorType == 3) {
            temp = "black";
        }
    }
    return temp;
});
const showMinArea = ref(false);
const userProvinceId = useCookie<any[]>("userProvinceId");
onMounted(() => {
    if (commonStore.topRegion.length === 0) {
        _getTopRegion();
        _getUserRegion();
    }
});
const _getTopRegion = async () => {
    try {
        const result = await getTopRegion();
        commonStore.topRegion = result ?? [];
    } catch (error) {}
};
const _getUserRegion = async () => {
    if (userProvinceId.value) return;
    try {
        const result = await getUserRegion();
        userProvinceId.value = result?.regionId ?? 0;
    } catch (error) {}
};
const regionname = computed(() => {
    return commonStore.topRegion.find((item) => item.regionId === userProvinceId.value)?.regionName ?? "";
});
const selectRegion = (id: number) => {
    userProvinceId.value = id;
};
</script>
<style lang="less" scoped>
.region-select {
    display: flex;
    align-items: center;
    flex: 1;
    position: relative;

    .item {
        display: flex;
        align-items: center;
        height: 31px;
        padding: 0 10px;
        border-left: 1px solid transparent;
        border-right: 1px solid transparent;
        position: relative;
        z-index: 12;
        cursor: pointer;
        .ico-color {
            color: var(--general);
        }

        div:first-child {
            color: #ff4040;
        }

        .cityname {
            font-size: 12px;
            color: #666;
            margin-left: 3px;
        }
    }

    .item.hover {
        background: #fff;
        border-left: 1px solid #dbdbdb !important;
        border-right: 1px solid #dbdbdb !important;
    }

    .item.hover .cityname {
        color: var(--general);
    }

    .dropdown-mini-area {
        position: absolute;
        border: 1px solid #dbdbdb;
        background-color: #fff;
        box-shadow: 1px 2px 1px rgba(0, 0, 0, 0.1);
        width: 300px;
        padding: 10px;
        line-height: 24px;
        top: 31px;
        z-index: 11;
        border-top: 0;
    }
    .areamini-content-list {
        display: flex;
        flex-wrap: wrap;
        overflow: hidden;
    }

    .areamini-content-list .cityitem {
        width: 60px;
        padding: 2px 0;
    }

    .areamini-content-list .cityitem a {
        float: left;
        padding: 0 8px;
    }

    .areamini-content-list .cityitem a:hover {
        background-color: #f4f4f4;
    }

    .areamini-content-list .cityitem a.selected {
        background-color: var(--general);
        color: var(--main-text) !important;
    }
}
</style>
