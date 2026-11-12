<template>
    <div class="filter__box">
        <div class="filter">
            <template v-if="filterList.attributes.length > 0">
                <div class="list" v-if="getUnselectedAttributes().length > 0">
                    <!-- 收起状态：显示未被选中的第一个筛选项 -->
                    <template v-if="!attrShowMore">
                        <div v-for="(item, index) in getFirstUnselectedAttribute()" :key="item.name" class="attr-list">
                            <div class="attr-name">{{ item.name }}：</div>
                            <div class="attr-box">
                                <div class="category-link">
                                    <div
                                        class="item"
                                        v-for="(value, key) in item.values"
                                        :class="{ selected: filterValueSelectedFun(item.name, value.value) }"
                                        :key="key"
                                        @click="onCategoryClick(item.name, value.value)"
                                    >
                                        {{ value.value }}
                                    </div>
                                </div>
                            </div>
                        </div>
                    </template>

                    <!-- 展开状态：显示所有未被选中的筛选项 -->
                    <template v-else>
                        <div v-for="(item, index) in getUnselectedAttributes()" :key="item.name" class="attr-list">
                            <div class="attr-name">{{ item.name }}：</div>
                            <div class="attr-box">
                                <div class="category-link">
                                    <div
                                        class="item"
                                        v-for="(value, key) in item.values"
                                        :class="{ selected: filterValueSelectedFun(item.name, value.value) }"
                                        :key="key"
                                        @click="onCategoryClick(item.name, value.value)"
                                    >
                                        {{ value.value }}
                                    </div>
                                </div>
                            </div>
                        </div>
                    </template>

                    <!-- 展开/收起按钮 -->
                    <span class="attrMore" :class="{ attrMoreUp: attrShowMore }" @click="attrShowMore = !attrShowMore" v-if="shouldShowExpandButton()">
                        {{ attrShowMore ? $t("收起筛选") : $t("展开筛选") }}
                        <i class="iconfont-pc icon-xiajiantou"></i>
                    </span>

                    <!-- 多选按钮 -->
                    <div class="attr-multiple" v-if="!attrMultiple && getUnselectedAttributes().length > 0" @click="enableMultipleSelection">
                        <i>＋</i>
                        <div class="attr-multiple-text">{{ $t("多选") }}</div>
                    </div>

                    <!-- 多选操作按钮 -->
                    <div class="fc-btn-box" v-if="attrShowMore && attrMultiple">
                        <a class="fc-btn-ok" :class="{ 'fc-disable': attrsList.length === 0 }" @click="onSubmitAttrMutiple">
                            {{ $t("确定") }}
                        </a>
                        <a class="fc-btn-cancel" @click="onColseAttrMultiple">
                            {{ $t("取消") }}
                        </a>
                    </div>
                </div>
            </template>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { ref } from "vue";
import { getSearchEsFilter } from "@/api/search/search";
import type { filterSeletedData, SearchFilter } from "@/types/search/search.d";
import { deepClone } from "@/utils/util";

// 响应式数据
const attrShowMore = ref(false);
const attrMultiple = ref(false);
const attrsList = ref<any[]>([]);
const filterList = ref<any>({
    attributes: []
});

// Props定义
const props = defineProps({
    queryParams: {
        type: Object,
        default: () => ({})
    },
    pageType: {
        type: String,
        default: "search"
    }
});

// Emits定义
const emit = defineEmits(["change"]);

// Models定义
const filter = defineModel<SearchFilter>("filter") as Ref<SearchFilter>;
const filterSeleted = defineModel<filterSeletedData>("filterSeleted") as Ref<filterSeletedData>;

// 计算未被选中的筛选项
const getUnselectedAttributes = () => {
    if (!filterList.value.attributes || filterList.value.attributes.length === 0) {
        return [];
    }
    return filterList.value.attributes.filter((item: any) => !filterSelectedFun(item.name));
};

// 获取第一个未被选中的筛选项
const getFirstUnselectedAttribute = () => {
    const unselected = getUnselectedAttributes();
    return unselected.length > 0 ? [unselected[0]] : [];
};

// 判断是否需要显示展开按钮
const shouldShowExpandButton = () => {
    const unselectedCount = getUnselectedAttributes().length;
    // 收起状态下，如果有超过1个未被选中的筛选项，则显示展开按钮
    if (!attrShowMore.value) {
        return unselectedCount > 1;
    }
    // 展开状态下，始终显示收起按钮
    return true;
};

// 启用多选模式
const enableMultipleSelection = () => {
    attrMultiple.value = true;
    attrShowMore.value = true;
};

// 判断筛选项是否已被选中
const filterSelectedFun = (name: string): boolean => {
    if (filterSeleted.value.attrs && filterSeleted.value.attrs.length > 0) {
        return filterSeleted.value.attrs.some((item) => item.attrName === name);
    }
    return false;
};

// 判断筛选值是否已被选中
const filterValueSelectedFun = (name: string, value: string): boolean => {
    if (!attrsList.value || attrsList.value.length === 0) {
        return false;
    }
    const attrItem = attrsList.value.find((item) => item.attrName === name);
    if (attrItem) {
        return attrItem.attrValue.includes(value);
    }
    return false;
};

// 获取搜索筛选项
const _getSearchFilter = async () => {
    try {
        const result = await getSearchEsFilter({ keyword: props.queryParams.keyword, cat: props.queryParams.cat });
        filterList.value.attributes = result.attributes || [];
    } catch (error) {
        console.error("获取筛选项失败:", error);
    }
};

// 监听筛选条件变化
watch(
    () => filter.value,
    () => {
        _getSearchFilter();
    },
    { deep: true, immediate: true }
);

// 监听已选筛选项变化
watch(
    () => filterSeleted.value,
    () => {
        attrsList.value = deepClone(filterSeleted.value.attrs) || [];
    },
    { deep: true, immediate: true }
);

// 分类点击处理
const onCategoryClick = (name: string, value: string) => {
    // 确保 attrsList.value 是数组
    if (!attrsList.value) {
        attrsList.value = [];
    }

    let attrItem = attrsList.value.find((item) => item.attrName === name);
    if (!attrItem) {
        // 如果attrsList中没有该属性项，则新增该项
        attrItem = { attrName: name, attrValue: [] };
        attrsList.value.push(attrItem);
    }

    const index = attrItem.attrValue.indexOf(value);
    if (index > -1) {
        // 如果已经选择，则取消选择
        attrItem.attrValue.splice(index, 1);
    } else {
        // 如果没有选择，则新增
        attrItem.attrValue.push(value);
    }

    if (!attrMultiple.value) {
        emit("change", attrsList.value);
    }
};

// 提交多选筛选
const onSubmitAttrMutiple = () => {
    if (attrsList.value.length > 0) {
        emit("change", attrsList.value);
    }
};

// 关闭多选模式
const onColseAttrMultiple = () => {
    attrsList.value = [];
    attrMultiple.value = false;
};
</script>

<style lang="less" scoped>
.sear-title {
    background: #f8f8f8 none repeat scroll 0 0;
    height: 34px;
    line-height: 34px;
    overflow: hidden;
    margin-top: 10px;
    display: flex;
    align-items: center;

    h3 {
        padding-left: 10px;
        font-weight: bold;
        font-size: 14px;

        b {
            color: var(--general);
            font-weight: bold;
            margin-right: 5px;
        }

        em {
            font-weight: bold;
        }
    }

    .st-ext {
        padding-left: 20px;
    }
}

.filter .list {
    position: relative;
    overflow: hidden;
    padding: 6px 0;
    height: 100%;
    border-bottom: 1px solid #dddddd;
    line-height: 20px;
    transition: height 0.3s ease 0s;
}

.filter .list .attr-list {
    display: flex;
    width: 100%;
    min-width: 0;
}

.filter .attr-name {
    flex: 0 0 80px;
    width: 80px;
    min-width: 80px;
    font-weight: normal;
    font-size: 12px;
    line-height: 28px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
}

.filter .attr-box {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    padding: 0 130px 0 0px;
}

.filter .attr-box .category-link {
    line-height: 28px;
    overflow: hidden;
    white-space: nowrap;

    .item {
        padding: 0;
        cursor: pointer;
    }
}

.filter .attr-box .category-link-more {
    height: 100%;
}

.filter .attr-box .item {
    display: inline-block;
    margin-right: 10px;
    padding: 0 8px;
    color: #767676;
    white-space: nowrap;
}

.filter .attr-box .item:hover {
    color: var(--general);
}

.filter .attr-box .selected {
    color: var(--general);
}

.attrMore {
    position: absolute;
    top: 10px;
    right: 60px;
    padding: 0 20px 0 0;
    cursor: pointer;

    i {
        margin-left: 3px;
        font-size: 12px;
    }
}

.attrMoreUp {
    i {
        transform: rotate(180deg);
        display: inline-block;
    }
}

.attr-multiple {
    position: absolute;
    top: 12px;
    right: 10px;
    min-width: 45px;
    max-width: 60px;
    min-height: 18px;
    border: 1px solid #dad9d9;
    text-align: center;
    font-weight: normal;
    line-height: 18px;
    cursor: pointer;
    display: flex;
    align-items: center;

    i {
        color: #aaaaaa;
        font-size: 16px;
    }

    .attr-multiple-text {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
}

.priceform {
    display: inline-flex;

    .form-bg {
        padding: 0 15px;
        width: 280px;
    }

    input {
        padding: 3px;
        width: 52px;
        height: 18px;
        border: 1px solid #eaeaea;
        vertical-align: top;
        font-size: 12px;
        line-height: 18px;
    }

    input:hover {
        border-color: var(--general);
    }
}

.btn-default {
    display: inline-block;
    margin-left: 3px;
    padding: 5px 13px 5px;
    height: 14px;
    border: 1px solid #ddd;
    border-radius: 2px;
    background: #f7f7f7;
    background-color: #f7f7f7;
    background-image: -moz-linear-gradient(to bottom, #f7f7f7, #f2f2f2);
    background-image: -webkit-linear-gradient(to bottom, #f7f7f7, #f2f2f2);
    background-image: -o-linear-gradient(to bottom, #f7f7f7, #f2f2f2);
    background-image: linear-gradient(to bottom, #f7f7f7, #f2f2f2);
    background-repeat: repeat-x;
    color: #666;
    text-align: center;
    text-decoration: none;
    line-height: 14px;
    cursor: pointer;
}

.btn-default:hover {
    box-shadow: 0 1px 1px rgba(0, 1, 1, 0.08);
    color: #666;
    text-decoration: none;
    cursor: pointer;
}

.filter .brand_fitlter .attr-box {
    margin-top: 10px;
}

.filter .brand_fitlter .attr-box .item {
    display: inline-block;
    margin-right: 8px;
    margin-left: 0;
    border: 1px solid #eeeeee;
    padding: 0;
    width: 110px;
    height: 45px;
    overflow: hidden;
    margin-bottom: 8px;
    position: relative;

    &.selected {
        border-color: var(--general);
    }

    &.selected i {
        display: block;
        height: 14px;
        overflow: hidden;
        position: absolute;
        right: 0px;
        bottom: 0;
        width: 14px;
        text-indent: -999px;
    }
}

.filter .brand_fitlter .attr-box .item img {
    height: 45px;
    margin: 0 auto;
    display: block;
    max-width: 110px;
}

.filter .brand_fitlter .attr-box .item {
    background: none;
}

.filter .brand_fitlter .attr-box .item:hover {
    border-color: var(--general);
}

.filter .brand_fitlter .attr-box .item i {
    display: none;
}

.filter .attr-box .brand_name {
    width: 110px;
    height: 45px;
    background: #fff;
    position: absolute;
    top: 0px;
    display: none;
    cursor: pointer;
    text-align: center;
    line-height: 45px;
    color: var(--general);
}

.filter .brand_fitlter .attr-box .item:hover .brand_name {
    display: block;
}

.category-brand-f-letter {
    width: 100%;
    color: var(--general);
    display: flex;
    flex-wrap: wrap;
    margin-bottom: 20px;

    .leter {
        border: 1px solid #fff;
        cursor: pointer;
        height: 22px;
        line-height: 22px;
        margin-right: 5px;
        padding: 0 8px;
    }
}

.category-brand-wrap {
    height: 55px;
}

.category-brand-f-wrap {
    width: 100%;
    display: flex;
    flex-wrap: wrap;
}

.category-brand-f-letter .cur {
    border: 1px solid var(--general);
    color: var(--general);
}

.fc-btn-box {
    line-height: 20px;
    padding: 13px 0 10px;
    display: flex;
    justify-content: center;
}

.fc-btn-ok {
    overflow: hidden;
    padding: 0 9px;
    height: 20px;
    background-color: var(--general);
    border: 1px solid var(--general);
    color: var(--main-text);
    vertical-align: middle;
    line-height: 20px;
    margin-right: 3px;
}

.fc-btn-ok:hover {
    color: var(--main-text) !important;
}

.fc-disable,
.fc-disable:hover {
    border-color: #e6e6e6;
    background: #f8f8f8 none repeat scroll 0 0;
    color: #ccc;
}

.fc-btn-cancel {
    overflow: hidden;
    padding: 0 9px;
    height: 20px;
    border: 1px solid #ddd;
    background: #fff none repeat scroll 0 0;
    color: #333;
    vertical-align: middle;
    line-height: 20px;
}

// 排序
.cat-sort-warp {
    margin: 10px 0;
}

.cat-sort {
    height: 32px;
    background: #f8f8f8;
    padding: 5px 0;
    padding-right: 10px;
    display: flex;
    align-items: center;
    justify-content: space-between;

    .lefter {
        background: #fff none repeat scroll 0 0;
        border: 1px solid #eee;
        height: 24px;
        margin-left: 8px;
        display: flex;
        align-items: center;

        .sort-link {
            padding: 0 20px;
            font-size: 12px;
            position: relative;
            height: 24px;
            display: flex;
            cursor: pointer;
            align-items: center;

            span {
                height: 24px;
                line-height: 24px;
            }

            .iconfont-pc {
                font-size: 10px;
                margin-left: 3px;
            }
        }

        .sort-link:hover {
            color: var(--general);
        }

        .cur_sort {
            background-color: var(--general);
            color: var(--main-text);
        }

        .cur_sort:hover {
            color: white;
        }
    }

    .righter {
        display: flex;
        align-items: center;

        .colour_scan {
            margin-right: 20px;
        }

        .productsNO {
            display: flex;
            align-items: center;

            em {
                color: var(--general);
                margin: 0 3px;
                font-weight: 700;
            }
        }

        .pageInfo {
            display: flex;
            align-items: center;
            color: #999;
            margin-left: 20px;

            .page_number {
                color: #999;
                font-size: 14px;
                padding-right: 10px;
            }

            .noPage {
                display: flex;
                font-size: 14px;
                background-color: #ffffff;
                border: 1px solid #cacaca;
                color: #767676;
                height: 24px;
                text-align: center;
                width: 30px;
                margin-left: -1px;
                align-items: center;
                justify-content: center;

                &:hover {
                    color: var(--general);
                }
            }

            .disabled {
                color: #ddd !important;
            }
        }
    }
}
</style>
