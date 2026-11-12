<template>
    <div class="tig-tips flex align-center">
        <div class="logo">
            <img src="/assets/images/common/mark-tip.png" />
        </div>
        <div class="tips_text" :class="[{ top: top }, { bottom: bottom }]">
            <i class="d-arr"></i>
            <div>
                <slot></slot>
            </div>
        </div>
    </div>
</template>
<script setup lang="ts">
import { toRefs, ref, computed, reactive } from "vue";
const props = defineProps({
    top: { type: Boolean, default: false },
    bottom: { type: Boolean, default: false }
});
</script>
<style lang="less" scoped>
.tig-tips {
    height: 100%;
    margin-left: 5px;
    opacity: 1;
    position: relative;
    .logo {
        width: 13px;
        height: 13px;
        position: absolute;
        top: 50%;
        margin-top: -7.5px;
        img {
            width: 100%;
            height: 100%;
        }
    }
    .tips_text {
        position: absolute;
        white-space: nowrap;
        background-color: #eee;
        z-index: 99;
        left: -10px;
        border: 1px solid #edd28b;
        background-color: #fffdee;
        color: #333;
        padding: 2px 6px 2px 7px;
        box-shadow: 0 0 2px 2px #eee;
        display: none;
        .d-arr {
            display: block;
            position: absolute;
            background-image: url("/assets/images/common/tips.gif");
            background-repeat: no-repeat;
            width: 11px;
            height: 6px;
            overflow: hidden;
            bottom: 24px;
            left: 10px;
            background-position: 0 0;
            z-index: 100;
        }
    }
    &:hover {
        .tips_text {
            display: block;
        }
    }
    .top {
        top: -30px;
        .d-arr {
            bottom: -6px;
            left: 10px;
            background-position: 0px -6px;
        }
    }
    .bottom {
        bottom: -30px;
        .d-arr {
            bottom: 24px;
            left: 10px;
        }
    }
}
</style>
