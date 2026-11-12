<template>
    <div class="select_region_box region-shell-ZjIyODJjbG4" ref="selectPop">
        <div class="flex align-center">
            <div class="select_region hand-pointer flex align-center" @click="onChangeShow">
                <div v-if="regionNames.length > 0">
                    <template v-for="(item, index) in regionNames">{{ item }}&nbsp;</template>
                </div>
                <div v-if="regionNames.length == 0">{{ $t("请选择地区") }}</div>
                <i class="iconfont-pc icon-xiajiantou"></i>
            </div>
        </div>
        <div class="select_popup" v-if="isShow">
            <div class="top flex justify-between">
                <div class="tabs flex">
                    <div class="tab" @click="showIndex = index" :class="{ cur: showIndex === index }" v-for="(item, index) in regionNames">
                        {{ item ? item : $t("请选择") }}
                    </div>
                </div>
                <div class="selectclose" @click.stop="close" v-if="regionList.length == showIndex + 1">×</div>
            </div>
            <template v-for="(item, index) in regionList">
                <div class="sec_childs" v-if="showIndex === index">
                    <span
                        class="sec_item"
                        v-for="region in item"
                        :class="{ active: regionSelected(region.regionId) }"
                        @click="onSelect(index, region.regionId)"
                        >{{ region.regionName }}</span
                    >
                </div>
            </template>
        </div>
    </div>
</template>
<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted, watch } from "vue";
import { getRegionByIds, getRegionListByRegionId } from "@/api/region";
const regionIds = defineModel<number[]>("modelValue", { default: [] });
const regionNamesModel = defineModel<string[]>("regionNames", { default: [] });
const props = defineProps<{ countryRegionId?: number }>();
const isShow = ref(false);
const regionList = ref<any[]>([]);
const showIndex = ref(0);

const emit = defineEmits(["change"]);

onMounted(() => {
    getRegions();
});

const { t } = useI18n();
const getRegions = async () => {
    try {
        if (regionIds.value.length === 0 && props.countryRegionId) {
            const rootChildren = await getRegionListByRegionId(props.countryRegionId);
            regionList.value = [Array.isArray(rootChildren) ? rootChildren : []];
            showIndex.value = 0;
            return;
        }
        let idString = regionIds.value.join();
        const result = await getRegionByIds(idString);
        regionList.value = Array.isArray(result) ? result : [];
        if (regionList.value.length > 1) {
            if (regionList.value[regionList.value.length - 1].length > 0) {
                showIndex.value = regionList.value.length - 1;
            } else {
                showIndex.value = regionList.value.length - 2;
            }
        } else {
            showIndex.value = 0;
        }
    } catch (error) {
    } finally {
    }
};

const regionSelected = (regionId: any) => {
    return regionIds.value.includes(Number(regionId));
};
const onChangeShow = () => {
    isShow.value = true;
    getRegions();
};

const onSelect = async (index: number, regionId: any) => {
    regionId = Number(regionId);
    if (index >= 0 && index <= regionIds.value.length) {
        regionIds.value[index] = regionId; // 更新对应索引的值
        regionIds.value.splice(index + 1); // 截取至指定索引，包含修改过的值
    }
    let children: any[] = [];
    try {
        const result = await getRegionListByRegionId(regionId);
        children = Array.isArray(result) ? result : [];
    } catch {
        // Domestic trees still have the legacy bulk endpoint as a fallback.
        await getRegions();
        children = regionList.value[index + 1] || [];
    }
    regionList.value.splice(index + 1);
    if (children.length > 0) regionList.value[index + 1] = children;
    if (children.length === 0) {
        isShow.value = false;
    } else {
        showIndex.value = index + 1;
    }
    emit("change");
};

const regionNames = computed(() => {
    let res: any = [];
    regionList.value.forEach((item: any, index) => {
        if (item.length > 0) {
            let selected = false;
            item.forEach((region: any) => {
                if (regionSelected(region.regionId)) {
                    res[index] = region.regionName;
                    selected = true;
                }
            });
            if (selected == false) {
                res[index] = t("请选择地区");
            }
        }
    });
    regionNamesModel.value = res.filter(Boolean);
    return res;
});

const reset = () => {
    regionIds.value = [];
    regionList.value = [];
    showIndex.value = 0;
};

const close = () => {
    if (isShow.value === true && regionIds.value.length != regionNames.value.length) {
        reset();
    }
    isShow.value = false;
};
const selectPop = ref<HTMLElement | null>(null);

const handleOutsideClick = (event: MouseEvent) => {
    if (selectPop.value && !selectPop.value.contains(event.target as Node)) {
        close();
    }
};

// 监听 regionIds 变化，当从外部更新时重新加载地区列表
watch(
    () => regionIds.value,
    (newVal, oldVal) => {
        if (newVal && newVal.length > 0 && JSON.stringify(newVal) !== JSON.stringify(oldVal)) {
            getRegions();
        }
    },
    { deep: true }
);

onMounted(() => {
    window.addEventListener("click", handleOutsideClick);
});

onUnmounted(() => {
    window.removeEventListener("click", handleOutsideClick);
});
</script>
<style lang="less" scoped>
.select_region_box {
    position: relative;
}

.select_region {
    padding: 0 8px 0 15px;
    height: 33px;
    border-radius: 3px;
    box-shadow: 0 0 0 1px #dcdfe6 inset;
    line-height: 30px;
    font-size: 14px;
    color: #666;

    i {
        font-size: 14px;
        margin-left: 10px;
    }
}

.tips {
    color: #999;
    margin-left: 10px;
}

.select_popup {
    position: absolute;
    top: -6px;
    left: -6px;
    background-color: #fff;
    z-index: 2;
    padding: 6px;
    font-size: 12px;
    border-radius: 4px;
    width: 450px;
    border: 1px solid #d9d9d9;

    .top {
        position: relative;
        border-left: 1px solid #e5e5e5;
        overflow: hidden;
        z-index: 9;
        height: 32px;

        .tabs {
            align-content: flex-start;

            .tab {
                margin-left: -1px;
                padding: 5px 15px;
                height: 20px;
                background-color: #f7f7f7;
                border-width: 1px;
                border-style: solid;
                border-color: #e5e5e5;
                line-height: 20px;
                border-bottom: none;
                cursor: pointer;
                color: #999;
            }

            .cur {
                background-color: #fff;
                border-bottom-color: #fff;
                height: 22px;
            }
        }

        .selectclose {
            color: #999;
            font-family: "Tahoma";
            font-size: 22px;
            margin-right: 5px;
            line-height: 27px;
            cursor: pointer;
        }
    }

    .sec_childs {
        position: relative;
        margin-top: -1px;
        border-top: 1px solid #e5e5e5;
        z-index: 8;
        padding: 10px 0;
        .sec_item {
            display: inline-block;
            padding: 0px 8px;
            margin: 0 5px 6px 0;
            white-space: nowrap;
            border-radius: 2px;
            color: #333;
            cursor: pointer;
            transition: none;
            line-height: 26px;
        }

        .active {
            background-color: var(--main-bg);
            color: var(--main-text);
        }
    }
}
</style>
