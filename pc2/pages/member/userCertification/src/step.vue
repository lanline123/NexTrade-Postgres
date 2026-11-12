<template>
    <div class="zent-steps zent-steps-breadcrumb">
        <div
            v-for="(item, index) in stepList"
            :class="{ 'zent-steps-item--finished': index + 1 == modelValue }"
            :style="{ width: 100 / stepList.length + '%' }"
            class="zent-steps-item"
        >
            <div class="zent-steps-step">{{ $t(item) }}</div>
        </div>
    </div>
</template>
<script lang="ts" setup>
/**
 * @description 步骤条组件
 * - 显示认证流程的步骤进度
 * - 支持自定义步骤名称和当前激活步骤
 */
import { ref } from "vue";

const props = defineProps({
    modelValue: { type: Number },
    type: { type: Number, default: 1 },
    stepList: { type: Array, default: [] }
});

const stepList = ref(["1. 资料填写", "2. 审核及公示"]);
</script>
<style lang="less" scoped>
.zent-steps {
    width: 100%;
    line-height: 1.5;
    font-size: 0;
    display: -ms-flexbox;
    display: flex;
    box-sizing: border-box;

    &-step {
        position: relative;
    }

    &-item {
        position: relative;
        display: inline-block;
        vertical-align: top;
        overflow: visible;
        -ms-flex: 1;
        flex: 1;
        cursor: default;

        &:last-child {
            -ms-flex: none;
            flex: none;
        }
    }
}

.zent-steps-breadcrumb {
    overflow: hidden;

    .zent-steps-item {
        padding: 0 8px;
        box-sizing: border-box;

        &:first-child {
            padding-left: 0;

            .zent-steps-step {
                &:before {
                    display: none;
                }
            }
        }

        &:last-child {
            padding-right: 0;

            .zent-steps-step {
                &:after {
                    display: none;
                }
            }
        }
    }

    .zent-steps-item .zent-steps-step {
        color: #333;
        background-color: #f5f6fa;
        height: 48px;
        line-height: 48px;
        font-size: 14px;
        text-align: center;
        padding: 0 10px;

        &:after {
            border-left-color: #f5f6fa;
            content: " ";
            display: block;
            width: 0;
            height: 0;
            border-top: 24px solid transparent;
            border-bottom: 24px solid transparent;
            border-left-width: 12px;
            border-left-style: solid;
            position: absolute;
            top: 50%;
            margin-top: -24px;
            right: -12px;
        }

        &:before {
            border-top-color: #f7f7f7;
            border-bottom-color: #f7f7f7;
            content: " ";
            display: block;
            width: 0;
            height: 0;
            border-top-width: 24px;
            border-top-style: solid;
            border-bottom-width: 24px;
            border-bottom-style: solid;
            border-left: 12px solid transparent;
            position: absolute;
            top: 50%;
            margin-top: -24px;
            left: -12px;
        }
    }

    .zent-steps-item.zent-steps-item--finished {
        &:hover {
            .zent-steps-step {
                background-color: var(--general);

                &:after {
                    border-left-color: var(--general);
                }
            }

            .zent-steps-step:before {
                border-top-color: var(--general);
                border-bottom-color: var(--general);
            }
        }

        &:active {
            .zent-steps-step {
                background-color: var(--general);

                &:after {
                    border-left-color: var(--general);
                }
            }

            .zent-steps-step:before {
                border-top-color: var(--general);
                border-bottom-color: var(--general);
            }
        }

        .zent-steps-step {
            background-color: var(--general);
            color: #fff;

            &:after {
                border-left-color: var(--general);
            }

            &:before {
                border-top-color: var(--general);
                border-bottom-color: var(--general);
            }
        }
    }
}
</style>
