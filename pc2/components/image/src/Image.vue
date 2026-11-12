<template>
    <el-image
        v-if="_image"
        :src="_image"
        :preview-src-list="[_imageBig]"
        fit="contain"
        preview-teleported
        :style="'height:' + _height + ';' + 'width:' + _width + ';cursor:pointer'"
        hide-on-click-modal
    />
</template>
<script setup lang="ts">
import { toRefs, ref, computed } from "vue";
import { imageFormat } from "@/utils/format";
const visible = ref<boolean>(false);
const loaded = ref<boolean>(false);
const props = defineProps({
    src: { type: String, default: "" },
    big: { type: String, default: "" },
    class: { type: String, default: "" },
    width: { type: [Number, String], default: "auto" },
    height: { type: [Number, String], default: "auto" }
});
const { src, big } = toRefs(props);
const _width = computed(() => {
    if (props.width === undefined) {
        return "auto";
    } else {
        return props.width + "px";
    }
});
const _height = computed(() => {
    if (props.height === undefined) {
        return "auto";
    } else {
        return props.height + "px";
    }
});
const _image = computed(() => {
    return imageFormat(src.value);
});
const _imageBig = computed(() => {
    return big.value ? imageFormat(big.value) : imageFormat(src.value);
});
const setVisible = (value): void => {
    loaded.value = true;
    visible.value = value;
};
</script>
