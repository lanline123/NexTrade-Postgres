<template>
    <div>
        <div class="countdown-box" v-if="currentCountdownValue">
            <span class="countdown-text">{{ $t("距离结束") }}: </span>
            <template v-if="formatTime.days && Number(formatTime.days) > 0">
                <span class="countdown-item" :class="{ 'countdown-item-big': formatTime.days && Number(formatTime.days) > 99}">{{ formatTime.days }}</span>
                <span class="countdown-separator-text">{{ $t("天") }}</span>
            </template>
            <span class="countdown-item">{{ formatTime.hours || '00' }}</span>
            <span class="countdown-separator">:</span>
            <span class="countdown-item">{{ formatTime.minutes || '00' }}</span>
            <span class="countdown-separator">:</span>
            <span class="countdown-item">{{ formatTime.seconds || '00' }}</span>
        </div>
        <div v-else>{{ $t("倒计时结束") }}</div>
    </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onBeforeUnmount } from "vue";

const specifyTime = defineModel<number>('specifyTime');
let timer: ReturnType<typeof setInterval> | null = null;

const currentCountdownValue = computed(() => {
    if (specifyTime.value && specifyTime.value < Date.now()) {
        return 0;
    }
    return specifyTime.value || 0;
});

const countdownTime = ref<number>(0);

const formatTime = computed(() => {
    const totalSeconds = Math.floor(countdownTime.value / 1000);
    const days = Math.floor(totalSeconds / (24 * 60 * 60));
    const hours = Math.floor((totalSeconds % (24 * 60 * 60)) / (60 * 60));
    const minutes = Math.floor((totalSeconds % (60 * 60)) / 60);
    const seconds = totalSeconds % 60;

    return {
        days: days.toString().padStart(2, "0"),
        hours: hours.toString().padStart(2, "0"),
        minutes: minutes.toString().padStart(2, "0"),
        seconds: seconds.toString().padStart(2, "0")
    };
});

onMounted(() => {
    if (!currentCountdownValue.value) return;

    // 立即执行一次计算
    const remainingTime = currentCountdownValue.value - Date.now();
    countdownTime.value = remainingTime > 0 ? remainingTime : 0;

    // 启动定时器
    timer = setInterval(() => {
        const remaining = currentCountdownValue.value - Date.now();
        if (remaining <= 0) {
            if (timer) clearInterval(timer);
            countdownTime.value = 0;
        } else {
            countdownTime.value = remaining;
        }
    }, 1000);
});

onBeforeUnmount(() => {
    if (timer) {
        clearInterval(timer);
        timer = null;
    }
});
</script>


<style lang="less" scoped>
.countdown-box {
    display: inline-block;
    .countdown-separator-text {
        font-size: 16px;
    }
    .countdown-text {
        font-size: 16px;
        font-weight: 400;
        line-height: 24px;
        color: #1a1a1a;
        text-transform: capitalize;
    }
    .countdown-item {
        display: inline-block;
        min-width: 30px;
        height: 30px;
        box-sizing: border-box;
        line-height: 30px;
        text-align: center;
        background-color: #1a1a1a;
        color: #fff;
        border-radius: 4px;
    }
    .countdown-item-big{
        padding: 0 5px;
    }

    .icon-youjiantou {
        color: #1a1a1a;
        font-size: 14px;
        padding-left: 4px;
    }

    span {
        margin: 0 1px;
    }
    .icon {
        margin-left: 8px;
    }
}
</style>
