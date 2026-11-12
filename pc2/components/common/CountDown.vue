<template>
    <div>
        <slot :days="formatTime.days" :hours="formatTime.hours" :minutes="formatTime.minutes" :seconds="formatTime.seconds"></slot>
    </div>
</template>
<script setup lang="ts">
import { computed, ref, onMounted, onBeforeUnmount, watch } from "vue";

const props = defineProps({
    countdownType: {
        type: String,
        default: "specifyTime"
    },
    specifyTime: {
        type: Number,
        default: 0
    }
});

let timer: ReturnType<typeof setInterval> | null = null;

const currentCountdownValue = computed(() => {
    if (props.specifyTime && props.specifyTime < Date.now()) {
        return 0;
    }
    return props.specifyTime || 0;
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

const updateCountdown = () => {
    if (!currentCountdownValue.value) {
        countdownTime.value = 0;
        return;
    }

    const remaining = currentCountdownValue.value - Date.now();
    if (remaining <= 0) {
        if (timer) clearInterval(timer);
        countdownTime.value = 0;
    } else {
        countdownTime.value = remaining;
    }
};

onMounted(() => {
    if (!currentCountdownValue.value) return;

    // 立即执行一次计算
    updateCountdown();

    // 启动定时器
    timer = setInterval(() => {
        updateCountdown();
    }, 1000);
});

watch(
    () => props.specifyTime,
    () => {
        if (timer) {
            clearInterval(timer);
        }
        updateCountdown();
        if (currentCountdownValue.value) {
            timer = setInterval(() => {
                updateCountdown();
            }, 1000);
        }
    }
);

onBeforeUnmount(() => {
    if (timer) {
        clearInterval(timer);
        timer = null;
    }
});
</script>
