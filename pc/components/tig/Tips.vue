<template>
    <div class="tips">
        <div v-if="tip.type === 'valid'" key="valid" class="circle">
            <div class="circle_right" />
        </div>
        <div v-else-if="tip.type === 'error'" key="error" class="message error">
            {{ tip.message }}
        </div>
        <div v-else-if="tip.type === 'info'" key="info" class="message info">
            {{ tip.message }}
        </div>
        <div v-else key="default" style="margin-left: 20px; width: 160px"></div>
    </div>
</template>
<script lang="ts" setup>
const props = defineProps({
    info: {
        type: Object,
        default: () => ({
            type: "",
            message: ""
        })
    }
});
const tip = computed(() => {
    return { type: props.info.type, message: props.info.message };
});
</script>
<style lang="less" scoped>
.tips {
    position: absolute;
    left: 100%; /* 小尖尖的位置，根据需要调整 */

    .message {
        margin-left: 20px; /* 可根据需要调整间距 */
        position: relative;
        width: 160px;
        flex-wrap: wrap;
        height: auto;
        border-radius: 8px; /* 气泡边缘圆角 */
        padding: 10px;
        box-sizing: border-box;
        /* 文字布局 */
        display: flex;
        justify-content: center;
        align-items: center;
        /* 文字样式 */
        font-size: 12px;
        color: #666;
        line-height: 20px;
    }

    .message:before {
        content: "";
        position: absolute;
        left: -16px; /* 小尖尖的位置，根据需要调整 */
        top: 50%; /* 将尖尖定位到父元素的正中间 */
        border-width: 8px; /* 三角形的大小 */
        border-style: solid;

        transform: translateY(-50%); /* 确保尖尖对准中心 */
    }

    .info {
        background-color: #f0f0f0; /* 气泡背景颜色 */
    }

    .info:before {
        border-color: transparent #f0f0f0 transparent transparent; /* 仅右侧边框为气泡背景颜色，其余透明 */
    }

    .error {
        background-color: rgb(255, 244, 215); /* 气泡背景颜色 */
    }

    .error:before {
        border-color: transparent rgb(255, 244, 215) transparent transparent; /* 仅右侧边框为气泡背景颜色，其余透明 */
    }

    .circle {
        width: 160px;
        margin-left: 20px; /* 可根据需要调整间距 */

        .circle_right {
            width: 23px;
            height: 23px;
            background: url("/assets/images/reg/regist_new.png") no-repeat -50px 0;
        }
    }
}
</style>
