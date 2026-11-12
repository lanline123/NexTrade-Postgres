<template>
    <svg :class="svgClass" v-bind="$attrs">
        <use :xlink:href="iconName" />
    </svg>
</template>

<script setup>
import { computed } from "vue";

const props = defineProps({
    name: {
        type: String,
        required: true
    },
    color: {
        type: String,
        default: ""
    }
});

const iconName = computed(() => `#icon-${props.name}`);
const svgClass = computed(() => {
    if (props.name) {
        return `icon icon-${props.name}`;
    }
    return "icon";
});
</script>

<style lang="less">
.icon {
}
</style>
