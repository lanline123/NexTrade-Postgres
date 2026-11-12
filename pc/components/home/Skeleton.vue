<template>
    <div :style="{ width: width, height: height }" class="animation"></div>
</template>
<script lang="ts" setup>
const props = defineProps({
    width: {
        type: [Number, String],
        default: "100%"
    },
    height: {
        type: [Number, String],
        default: "100%"
    }
});
const width = ref();
if (typeof props.width == "number") {
    width.value = props.width + "px";
} else {
    width.value = width.value ?? "100%";
}
const height = ref();
if (typeof props.height == "number") {
    height.value = props.height + "px";
} else {
    height.value = height.value ?? "100%";
}
</script>
<style lang="less" scoped>
.animation {
    border-radius: 0;
    background-color: #f2f2f2;
    //animation: loading 2s ease infinite;
    //background: linear-gradient(90deg, #f0f2f5 25%, #e6e8eb 37%, #f0f2f5 63%);
    background-size: 400% 100%;
}

@keyframes loading {
    0% {
        background-position: 100% 50%;
    }
    100% {
        background-position: 0 50%;
    }
}
</style>
