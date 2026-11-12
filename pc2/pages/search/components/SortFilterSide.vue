<template>
    <div class="sort-filter-wrap">
        <div class="sort-item" :class="{ active: filter.sortField == '' }" @click="changeSort('')">
            <div class="sort-name">{{ $t("推荐") }}</div>
            <div class="sort-icon">
                <span class="tri down" :class="{ active: filter.sortField == '' }"></span>
            </div>
        </div>
        <div class="sort-item" :class="{ active: filter.sortField == 'price' }" @click="changeSort('price')">
            <div class="sort-name">{{ $t("价格") }}</div>
            <div class="sort-icon">
                <span class="tri up" :class="{ active: filter.sortField == 'price' && filter.sortOrder == 'asc' }"></span>
                <span class="tri down" :class="{ active: filter.sortField == 'price' && filter.sortOrder == 'desc' }"></span>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
import type { Ref } from "vue";
import type { SearchFilter } from "./types.d";

const emit = defineEmits(["change"]);
const filter = defineModel<SearchFilter>("filter", { default: () => ({ brandIds: [], attrs: [] }) }) as Ref<SearchFilter>;

const changeSort = (sort: string): void => {
    if (sort === "") {
        filter.value.sortField = "";
        filter.value.sortOrder = "";
    } else if (sort === "price") {
        filter.value.sortField = "price";
        filter.value.sortOrder = filter.value.sortOrder === "asc" ? "desc" : "asc";
    }
    filter.value.page = 1;
    emit("change");
};
</script>

<style lang="less" scoped>
.sort-filter-wrap {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    width: 100%;
    border-radius: 8px;
    background-color: #f7f8fc;
    padding: 8px 16px;
    margin-bottom: 24px;
    height: 40px;

    .sort-item {
        cursor: pointer;
        display: flex;
        align-items: center;
        height: 40px;

        &:nth-child(n + 2) {
            margin-left: 24px;
        }

        &.active {
            .sort-name {
                font-weight: 700;
                text-transform: capitalize;
            }
        }

        .sort-name {
            color: #1a1a1a;
            font-size: 16px;
        }

        .sort-icon {
            display: flex;
            flex-direction: column;
            justify-content: center;
            row-gap: 2px;
            line-height: 21px;
            margin-left: 4px;
            padding-top: 5px;

            .tri {
                width: 0;
                height: 0;
                border-left: 4px solid transparent;
                border-right: 4px solid transparent;
                border-top: 4px solid #c2c4cc;
            }

            .up {
                transform: rotate(180deg) translateY(1px);

                &.active {
                    border-top-color: #1a1a1a;
                }
            }

            .down {
                &.active {
                    border-top-color: #1a1a1a;
                }
            }
        }
    }
}
</style>
