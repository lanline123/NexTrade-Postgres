<template>
    <div v-if="!loading" class="track-list-date" :style="{ height: filterState?.length > 0 ? '400px' : '50px' }">
        <div class="bold" v-if="shippingMethod == 1">{{ $t("快递公司") }}：【{{ logisticsName }}】，{{ $t("快递单号") }}：【{{ trackingNo }}】</div>
        <div class="bold" v-else>{{ $t("快递类型") }}：{{ logisticsName }}</div>
        <template v-if="filterState?.length > 0">
            <div class="node-to-change" v-show="!loading">
                <ul>
                    <li v-for="(item, index) in filterState" :class="{ first: index === 0 && item.checkpointStatus === 'delivered' }">
                        <i class="status status-first status-check"></i>
                        <span :class="{ 'first-color': index === 0 }">{{ item.acceptStation }}</span>
                        <span :class="{ 'first-color': index === 0 }" class="date">{{ item.acceptTime }}</span>
                    </li>
                </ul>
            </div>
        </template>
        <div class="no-data" v-else>
            {{ $t("查无信息，请刷新后重试或者联系客服！") }}
        </div>
    </div>
    <div class="loading-data" v-else>
        {{ $t("读取数据中，请稍候片刻...") }}
    </div>
</template>

<script lang="ts" setup>
import { onMounted, ref } from "vue";
import { getShippingInfo } from "~/api/user/order";
import type { ShippingInfoTrace } from "~/types/user/order.d";

const props = defineProps({
    //是否显示尾部上半部分 1显示 2不显示
    id: {
        type: Number,
        default: 0
    },
    trackingNo: {
        type: String,
        default: ""
    },
    logisticsName: {
        type: String,
        default: ""
    },
    shippingMethod: {
        type: Number,
        default: 1
    }
});

onMounted(() => {
    console.log("mounted");
    loadFilter();
});
const filterState = ref<ShippingInfoTrace[]>([]);
const loading = ref<boolean>(true);

// 获取列表的查询结果
const loadFilter = async () => {
    loading.value = true;
    try {
        const result = await getShippingInfo({ id: props.id });
        if (result) {
            if (Array.isArray(result)) {
                filterState.value = result;
            } else {
                filterState.value = result.traces ?? ([] as ShippingInfoTrace[]);
            }
        }
    } catch (error: any) {
        message.error(error.message);
    } finally {
        loading.value = false;
    }
};
</script>

<style lang="less" scoped>
.track-list-date {
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    padding: 10px;
}
.loading-data {
    width: 100%;
    height: 50px;
    display: flex;
    justify-content: center;
    align-items: center;
}
.no-data {
    width: 100%;
    height: 50px;
    display: flex;
    justify-content: center;
    align-items: center;
}

.node-to-change {
    padding: 5px 25px 15px 38px;
}

li {
    padding: 10px 0;
    position: relative;

    /* 为除最后一个li外的每个li添加竖线 */
    &:not(:last-child)::after {
        content: "";
        background: #3985ff;
        width: 1px;
        height: 100%; /* 使得线条的长度恰好达到下一个图标的顶部 */
        position: absolute;
        left: -20px;
        top: 20px; /* 调整以确保线条从图标中心开始 */
    }
}
.first-color {
    color: black;
}

.status {
    background: #fff;
    border: 1px solid #3985ff;
    border-radius: 8px;
    height: 8px;
    width: 8px;
    position: absolute;
    left: -24px;
    top: 13px;
    z-index: 1;

    &::after {
        background: #3985ff;
        border-radius: 6px;
        content: "";
        display: block;
        height: 6px;
        width: 6px;
        position: absolute;
        left: 50%;
        top: 50%;
        transform: translate(-50%, -50%);
    }
}

.first .status {
    border-color: #fb8a00;

    &::after {
        background-color: #fb8a00;
    }
}

.date {
    display: block;
    margin-top: 2px; /* 通过CSS管理间距 */
}
.bold {
    font-weight: bold;
}
</style>
