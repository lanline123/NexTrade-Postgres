<template>
    <Modal
        class="verifybox"
        v-model:open="visible"
        :z-index="3000"
        centered
        :width="parseInt(imgSize.width) + 30 + 'px'"
        :title="$t('请完成安全验证')"
        :destroyOnClose="false"
        :footer="null"
    >
        <VerifySlide
            :is="componentType"
            :captchaType="captchaType"
            type="2"
            :figure="figure"
            :arith="arith"
            :mode="mode"
            :vSpace="vSpace"
            :explain="explain"
            :imgSize="imgSize"
            :blockSize="blockSize"
            :barSize="barSize"
            ref="instance"
            @close="closeBox"
            @success="success"
        ></VerifySlide>
    </Modal>
</template>
<script lang="ts" setup>
/**
 * Verify 验证码组件
 * @description 分发验证码使用
 * */
import VerifySlide from "./Verify/VerifySlide.vue";
import { Modal } from "ant-design-vue";
import { computed, ref, onMounted, onBeforeUnmount, toRefs, watch } from "vue";

const props = defineProps({
    captchaType: {
        type: String,
        required: true
    },
    figure: {
        type: Number
    },
    arith: {
        type: Number
    },
    mode: {
        type: String,
        default: "pop"
    },
    vSpace: {
        type: Number
    },
    explain: {
        type: String
    },
    imgSize: {
        type: Object,
        default() {
            return {
                width: "310px",
                height: "155px"
            };
        }
    },
    blockSize: {
        type: Object
    },
    barSize: {
        type: Object
    },
    close: {
        type: Function
    }
});
const { captchaType, figure, arith, mode, vSpace, explain, imgSize, blockSize, barSize } = toRefs(props);
const clickShow = ref(false);
const verifyType = ref(undefined);
const componentType = ref(undefined);

const instance = <any>ref({});
const emit = defineEmits([
    "okCallback" //确认后回调
]);
const visible = ref(false);
/**
 * refresh
 * @description 刷新
 * */
const refresh = () => {
    if (instance.value.refresh) {
        instance.value.refresh();
    }
};
const closeBox = () => {
    visible.value = false;
    refresh();
};
// 记录打开前的 body overflow，关闭或卸载时恢复，避免路由跳转后页面无法滚动
let prevBodyOverflow: string | null = null;
const lockBodyScroll = () => {
    if (prevBodyOverflow === null) {
        prevBodyOverflow = document.body.style.overflow;
    }
    document.body.style.overflow = "hidden";
};
const unlockBodyScroll = () => {
    if (prevBodyOverflow === null) return;
    document.body.style.overflow = prevBodyOverflow;
    prevBodyOverflow = null;
};
watch(visible, (val) => {
    if (!val) unlockBodyScroll();
});
onBeforeUnmount(unlockBodyScroll);
const show = () => {
    lockBodyScroll();
    visible.value = true;
};
const success = (res: any) => {
    emit("okCallback", res);
};
defineExpose({
    show
});
</script>
<style lang="less">
.verifybox .ant-modal-content {
    padding: 0;
}
.verifybox .ant-modal-body {
    padding: 15px !important;
}
.verifybox .ant-modal-header {
    padding: 16px 24px;
    border-bottom: 1px solid rgba(5, 5, 5, 0.06);
    margin-bottom: 0;
}

.verifybox .ant-modal-header {
    padding: 16px 15px !important;
}

.verifybox-top {
    padding: 0 15px;
    height: 50px;
    line-height: 50px;
    text-align: left;
    font-size: 16px;
    color: #45494c;
    border-bottom: 1px solid #e4e7eb;
    box-sizing: border-box;
}

.verifybox-bottom {
    padding: 15px;
    box-sizing: border-box;
}

.verifybox-close {
    position: absolute;
    top: 13px;
    right: 9px;
    width: 24px;
    height: 24px;
    text-align: center;
    cursor: pointer;
}

.mask {
    position: fixed;
    top: 0;
    left: 0;
    z-index: 1001;
    width: 100%;
    height: 100vh;
    background: rgba(0, 0, 0, 0.3);
    /* display: none; */
    transition: all 0.5s;
}

.verify-tips {
    position: absolute;
    left: 0px;
    bottom: 0px;
    width: 100%;
    height: 0;
    transition: all 0.2s linear;
    line-height: 30px;
    text-indent: 10px;
    color: #fff;
}

.verify-tips.show {
    height: 30px;
}

.suc-bg {
    background-color: rgba(92, 184, 92, 0.5);
    filter: progid:DXImageTransform.Microsoft.gradient(startcolorstr=#7f5CB85C, endcolorstr=#7f5CB85C);
}

.err-bg {
    background-color: rgba(217, 83, 79, 0.5);
    filter: progid:DXImageTransform.Microsoft.gradient(startcolorstr=#7fD9534F, endcolorstr=#7fD9534F);
}

.tips-enter,
.tips-leave-to {
    bottom: -30px;
}

.tips-enter-active,
.tips-leave-active {
    transition: bottom 0.5s;
}

/* ---------------------------- */
/*常规验证码*/
.verify-code {
    font-size: 20px;
    text-align: center;
    cursor: pointer;
    margin-bottom: 5px;
    border: 1px solid #ddd;
}

.cerify-code-panel {
    height: 100%;
    overflow: hidden;
}

.verify-code-area {
    float: left;
}

.verify-input-area {
    float: left;
    width: 60%;
    padding-right: 10px;
}

.verify-change-area {
    line-height: 30px;
    float: left;
}

.varify-input-code {
    display: inline-block;
    width: 100%;
    height: 25px;
}

.verify-change-code {
    color: #337ab7;
    cursor: pointer;
}

.verify-btn {
    width: 200px;
    height: 30px;
    background-color: #337ab7;
    color: #ffffff;
    border: none;
    margin-top: 10px;
}

/*滑动验证码*/
.verify-bar-area {
    position: relative;
    background: #f7f9fa;
    text-align: center;
    box-sizing: content-box;
    border: 1px solid #ddd;
    border-radius: 4px;
}

.verify-bar-area .verify-move-block {
    position: absolute;
    top: 0px;
    left: 0;
    background: #fff;
    color: #666;
    cursor: pointer;
    box-sizing: content-box;
    box-shadow: 0 0 3px rgba(0, 0, 0, 0.3);
    transition: background 0.2s linear;
    border-radius: 1px;
    border: 0;
}

.verify-bar-area .verify-move-block:hover {
    background-color: #1991fa;
    color: #ffffff;
}

.verify-bar-area .verify-left-bar {
    position: absolute;
    top: -1px;
    left: -1px;
    background: #d1e0f1;
    cursor: pointer;
    -webkit-box-sizing: content-box;
    -moz-box-sizing: content-box;
    box-sizing: content-box;
    border: 1px solid #ddd;
    border-radius: 0 2px 2px 0;
}

.verify-img-panel {
    margin: 0;
    -webkit-box-sizing: content-box;
    -moz-box-sizing: content-box;
    box-sizing: content-box;
    border-radius: 3px;
    position: relative;
}

.verify-img-panel .verify-refresh {
    width: 25px;
    height: 25px;
    text-align: center;
    padding: 5px;
    cursor: pointer;
    position: absolute;
    top: 0;
    right: 0;
    z-index: 2;
}

.verify-img-panel .icon-refresh {
    font-size: 20px;
    color: #fff;
    text-shadow: 1px 1px 3px rgba(0, 0, 0, 0.65);
}

.verify-img-panel .verify-gap {
    background-color: #fff;
    position: relative;
    z-index: 2;
    border: 1px solid #fff;
}

.verify-bar-area .verify-move-block .verify-sub-block {
    position: absolute;
    text-align: center;
    z-index: 3;
    /* border: 1px solid #fff; */
}

.verify-bar-area .verify-move-block .verify-icon {
    font-size: 18px;
}

.verify-bar-area .verify-msg {
    z-index: 3;
}

.icon-check:before {
    content: "\e6e9";
    display: block;
    width: 16px;
    height: 16px;
    line-height: 16px;
    position: absolute;
    margin: auto;
    left: 0;
    right: 0;
    top: 0;
    bottom: 0;
    z-index: 9999;
    background-size: contain;
}

.icon-close:before {
    content: "\e6bb";
    display: block;
    width: 16px;
    height: 16px;
    line-height: 16px;
    position: absolute;
    margin: auto;
    left: 0;
    right: 0;
    top: 0;
    bottom: 0;
    z-index: 9999;
    background-size: contain;
}

.icon-right:before {
    content: "\e676";
    display: block;
    width: 16px;
    height: 16px;
    position: absolute;
    line-height: 16px;
    margin: auto;
    left: 0;
    right: 0;
    top: 0;
    bottom: 0;
    background-size: cover;
    z-index: 9999;
    background-size: contain;
}

.icon-refresh:before {
    content: "\e616";
    display: block;
    width: 16px;
    height: 16px;
    line-height: 16px;
    position: absolute;
    margin: auto;
    left: 0;
    right: 0;
    top: 0;
    bottom: 0;
    z-index: 9999;
    background-size: contain;
}
</style>
