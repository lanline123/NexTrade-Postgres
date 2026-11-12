<template>
    <div v-if="show" ref="soldOutRef" class="sold-out">
        <div class="sold-out-text" :class="{ 'is-new-ui': isNewUi }" :style="{ fontSize: fontSize + 'px' }">
            {{ text }}
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue';
import { getProductStatusText, shouldShowProductStatus } from '~/utils/productSoldOut';

const props = defineProps<{
    productStock: number;
    productStatus: number;
    productType?: number;
    deliveryMode?: number;
    deliveryType?: number;
    stockField?: string;
    statusField?: string;
    uiVersion?: string;
}>();

const soldOutRef = ref<HTMLElement | null>(null);
const fontSize = ref(16);

const isNewUi = computed(() => props.uiVersion === "new");

const show = computed(() => {
    return shouldShowProductStatus(props, {
        stockField: props.stockField,
        statusField: props.statusField
    });
});

const text = computed(() => {
    return getProductStatusText(props, {
        stockField: props.stockField,
        statusField: props.statusField
    });
});

const calculateFontSize = () => {
    if (!soldOutRef.value) return;

    const width = soldOutRef.value.offsetWidth;
    let size = 12;

    const sizeMap = [
        { max: 100, size: 12 },
        { max: 200, size: 14 },
        { max: 300, size: 16 },
        { max: 400, size: 20 },
        { max: Infinity, size: 24 }
    ];
    const matched = sizeMap.find(item => width < item.max);
    fontSize.value = matched ? Math.round(matched.size) : 16; // 默认值设为16
};

let resizeObserver: ResizeObserver | null = null;

onMounted(() => {
    if (typeof window !== 'undefined' && soldOutRef.value) {
        calculateFontSize();

        // 使用 ResizeObserver 监听容器大小变化
        if (window.ResizeObserver) {
            resizeObserver = new ResizeObserver(() => {
                calculateFontSize();
            });
            resizeObserver.observe(soldOutRef.value);
        } else {
            // 降级方案：使用 window resize 事件
            window.addEventListener('resize', calculateFontSize);
        }
    }
});

onUnmounted(() => {
    if (resizeObserver && soldOutRef.value) {
        resizeObserver.unobserve(soldOutRef.value);
        resizeObserver.disconnect();
    }
    if (typeof window !== 'undefined') {
        window.removeEventListener('resize', calculateFontSize);
    }
});
</script>

<style scoped lang="less">
.sold-out {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    z-index: 10;
    background-color: rgba(255, 255, 255, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;

    .sold-out-text {
        width: 70%;
        height: 70%;
        border-radius: 50%;
        background-color: rgba(0, 0, 0, 0.5);
        color: #fff;
        font-size: 20px;
        display: flex;
        align-items: center;
        justify-content: center;

        &.is-new-ui {
            width: 250px;
            height: 250px;
        }
    }
}
</style>

