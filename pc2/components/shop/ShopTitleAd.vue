<template>
    <TotalContainer :module="module.frame">
        <Title :module="temp"></Title>
    </TotalContainer>
</template>
<script setup lang="ts">
import { urlFormat } from "~/utils/format";
import TotalContainer from "~/components/shop/src/TotalContainer.vue";
import Title from "~/components/shop/src/Title.vue";

const props = defineProps({
    module: { type: Object, default: {} }
});

const temp = computed(() => {
    let val = props.module.title;
    val.showTitle = 1;
    return val;
});
</script>
<style scoped lang="less">
.base-class {
    display: flex;
    flex-direction: column;
    align-items: center;
    box-sizing: border-box;

    .top-bar {
        display: flex;
        align-items: center;
        justify-content: space-between;
        flex-direction: row;
        width: 100%;
        padding: 5px 10px 3px;
        box-sizing: border-box;

        .left-title {
            display: flex;
            align-items: center;
            gap: 4px;
            width: 100%;

            .image-size {
                width: 30px;
                height: 30px;
                margin-left: 20px;
            }

            .big {
                font-size: 16px;
                margin-right: 5px;
                font-weight: bold;
            }

            .sm {
                color: #aaa;
                font-size: 12px;
            }
        }

        .right-title {
            cursor: pointer;
            width: 60px;
            text-align: center;
        }
    }
}
</style>
