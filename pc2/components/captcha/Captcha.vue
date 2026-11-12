<template>
    <span class="captcha-img" :style="'width:' + props.width + 'px;height:' + props.height + 'px'">
        <img v-if="imageSrc" @click="onClick" :src="imageSrc" :width="props.width" :height="props.height" />
    </span>
</template>
<script setup lang="ts">
import request from "../../utils/request";
import { ref } from "vue";
const props = defineProps({
    width: {
        type: Number,
        default: 100
    },
    height: {
        type: Number,
        default: 25
    },
    range: {
        type: String,
        default: ""
    }
});
const width = ref(props.width);
const height = ref(props.height);
const imageSrc = ref("");
const captchaUid = ref("");
const emit = defineEmits(["update:modelValue"]);
const onClick = () => {
    request({
        url: "verification/tpCaptcha",
        method: "get",
        params: { range: props.range }
    }).then((result: any) => {
        imageSrc.value = result.data;
        captchaUid.value = result.uuid;
        emit("update:modelValue", captchaUid.value);
    });
};
onClick();

defineExpose({
    onClick
});
</script>

<style lang="less" scoped>
.captcha-img {
    cursor: pointer;
    display: inline-block;
}
</style>
