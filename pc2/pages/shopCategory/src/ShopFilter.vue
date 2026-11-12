<!-- 店铺分类页排序和价格区间筛选。 -->
<template>
    <div class="cat-sort-warp">
        <div class="cat-sort">
            <div class="flex">
                <div class="lefter">
                    <span class="sort-link" :class="{ cur_sort: filter.sortField == '' }" :title="t('默认排序（根据相关性排序）')" @click="changeSort('')">
                        {{ $t("默认排序") }}
                    </span>
                    <span class="sort-link" :class="{ cur_sort: filter.sortField == 'sale' }" :title="t('销量（销量从高到低）')" @click="changeSort('sale')">
                        {{ $t("销量") }}
                    </span>
                    <span
                        class="sort-link"
                        :class="{ cur_sort: filter.sortField == 'time' }"
                        :title="t('上架时间（最新发布排序）')"
                        @click="changeSort('time')"
                    >
                        {{ $t("新品") }}
                    </span>
                    <span
                        class="sort-link"
                        :class="{ cur_sort: filter.sortField == 'price' }"
                        :title="filter.sortOrder == 'desc' ? t('价格（价格从高到低）') : t('价格（价格从低到高）')"
                        @click="changeSort('price')"
                    >
                        {{ $t("价格") }}
                        <span v-if="filter.sortOrder == 'desc'" class="iconfont-pc icon-xiajiantou1"></span>
                        <span v-else class="iconfont-pc icon-shangjiantou1"></span>
                    </span>
                </div>
                <div class="priceform">
                    <div class="form-bg">
                        <input type="text" name="pricemin" v-model.number="minPrice" />
                        <em> - </em>
                        <input type="text" name="pricemax" v-model.number="maxPrice" />
                        <a class="btn-default" draggable="false" @click="onPriceChange" rel="nofollow" href="javascript:;">{{ $t("确定") }}</a>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

const emit = defineEmits(["change"]);
const filter = defineModel<any>("filter", { required: true });

const minPrice = ref<number | undefined>(filter.value.minPrice);
const maxPrice = ref<number | undefined>(filter.value.maxPrice);

// 同步 filter 中的价格到本地 ref
watch(
    () => filter.value,
    (newVal) => {
        if (newVal.minPrice !== minPrice.value) minPrice.value = newVal.minPrice;
        if (newVal.maxPrice !== maxPrice.value) maxPrice.value = newVal.maxPrice;
    },
    { deep: true }
);

// 排序
const changeSort = (sort: string) => {
    if (sort === "") {
        filter.value.sortField = "";
        filter.value.sortOrder = "";
    } else if (sort === "sale") {
        filter.value.sortField = "sale";
        filter.value.sortOrder = "";
    } else if (sort === "time") {
        filter.value.sortField = "time";
        filter.value.sortOrder = "";
    } else if (sort === "price") {
        filter.value.sortField = "price";
        filter.value.sortOrder = filter.value.sortOrder === "asc" ? "desc" : "asc";
    }
    filter.value.page = 1;
    emit("change");
};

// 价格
const onPriceChange = () => {
    if (minPrice.value !== undefined && maxPrice.value !== undefined && (minPrice.value > maxPrice.value || maxPrice.value === 0)) {
        return message.error(t("请输入正确的价格范围"));
    }
    filter.value.minPrice = minPrice.value ?? 0;
    filter.value.maxPrice = maxPrice.value ?? 0;
    filter.value.page = 1;
    emit("change");
};
</script>

<style lang="less" scoped>
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
    .flex {
        display: flex;
        align-items: center;
    }
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
}
.priceform {
    display: inline-flex;
    .form-bg {
        padding: 0 15px;
        width: 280px;
        display: flex;
        align-items: center;
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
    em {
        margin: 0 5px;
    }
}
.btn-default {
    display: inline-block;
    margin-left: 3px;
    padding: 0 13px;
    height: 26px;
    border: 1px solid #ddd;
    border-radius: 2px;
    background: #f7f7f7;
    background-color: #f7f7f7;
    background-image: linear-gradient(to bottom, #f7f7f7, #f2f2f2);
    background-repeat: repeat-x;
    color: #666;
    text-align: center;
    text-decoration: none;
    line-height: 26px;
    cursor: pointer;
}

.btn-default:hover {
    box-shadow: 0 1px 1px rgba(0, 1, 1, 0.08);
    color: #666;
    text-decoration: none;
    cursor: pointer;
}
</style>
