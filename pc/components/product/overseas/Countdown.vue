<template>
    <div class="countdown" v-if="timeLeft">
        {{ $t("距离结束") }} <span>{{ formatTime(timeLeft.days) }}</span> {{ $t("天") }} <span>{{ formatTime(timeLeft.hours) }}</span> :
        <span>{{ formatTime(timeLeft.minutes) }}</span> : <span>{{ formatTime(timeLeft.seconds) }}</span>
    </div>
    <div v-else>{{ $t("倒计时结束") }}</div>
</template>
<script lang="ts" setup>
import { ref, onMounted, onUnmounted } from "vue";
const props = defineProps({
    endTime: {
        type: String,
        default: ""
    }
});
const endDate = new Date(props.endTime).getTime();
const timeLeft = ref<any>(calculateTimeLeft());
let timer: ReturnType<typeof setInterval> | null = null;

function calculateTimeLeft() {
    const now = new Date().getTime();
    const difference = endDate - now;

    if (difference > 0) {
        const days = Math.floor(difference / (1000 * 60 * 60 * 24));
        const hours = Math.floor((difference % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
        const minutes = Math.floor((difference % (1000 * 60 * 60)) / (1000 * 60));
        const seconds = Math.floor((difference % (1000 * 60)) / 1000);

        return { total: difference, days, hours, minutes, seconds };
    } else {
        return null;
    }
}
const formatTime = (time: number) => {
    return time < 10 ? `0${time}` : time;
};
onMounted(() => {
    timer = setInterval(() => {
        timeLeft.value = calculateTimeLeft();
        if (!timeLeft.value || timeLeft.value.total <= 0) {
            clearInterval(timer!);
            timer = null;
        }
    }, 1000);
});

onUnmounted(() => {
    if (timer) clearInterval(timer);
});
</script>

<style lang="less" scoped>
.countdown {
    color: #888b94;
    span {
        // background-color: #423b3b;
        font-size: 16px;
        line-height: 20px;
        // padding: 0px 8px;
    }
}
</style>
