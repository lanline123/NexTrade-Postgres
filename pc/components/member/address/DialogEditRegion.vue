<template>
    <div @click="show" :class="'dialog-link ' + props.class" :style="props.style">
        <slot></slot>
    </div>
    <template v-if="loaded">
        <el-dialog
            :class="dialogClass"
            v-model="visible"
            :width
            :title="_title"
            :destroyOnClose="true"
            @ok="onOk"
            v-bind="bindParams"
            :close-on-click-modal="maskClosable"
            :show-close="closable"
            centered
        >
            <MemberAddressEditRegion
                ref="submitForm"
                @submitCallback="submitCallback"
                @callback="callback"
                @show="show"
                @close="close"
                @okType="okType"
                v-bind="params"
                v-model:confirmLoading="confirmLoading"
            >
            </MemberAddressEditRegion>
        </el-dialog>
    </template>
</template>
<script setup lang="ts">
import { ref, computed } from "vue";

const props = defineProps({
    path: String, //路径
    class: {
        //样式
        type: String,
        default: ""
    },
    dialogClass: {
        //
        type: String,
        default: ""
    },
    params: {
        //绑定给页面的参数
        type: Object,
        default: {}
    },
    width: {
        //绑定给页面的参数
        type: String,
        default: "800px"
    },
    title: {
        //绑定给页面的参数
        type: String,
        default: ""
    },
    isDrawer: {
        type: Boolean,
        default: false
    },
    showClose: {
        type: Boolean,
        default: true
    },
    showOnOk: {
        type: Boolean,
        default: true
    },
    showFooter: {
        type: Boolean,
        default: true
    },
    data: {
        //额外传参，添加相册返回的同时会回传
        type: Object,
        default: {}
    },
    style: {
        //绑定给页面的参数
        type: [String, Object],
        default: "display:inline-flex;"
    },
    maskClosable: {
        type: Boolean,
        default: true
    },
    closable: {
        type: Boolean,
        default: true
    },
    type: String //类型  空:普通表单   gallery:相册
});

const emit = defineEmits([
    "okCallback", //确认后回调
    "callback"
]);

const loaded = ref(false);

const visible = ref<boolean>(false);
const confirmLoading = ref<boolean>(false);
const okDisabled = ref<boolean>(false);
const dialogClass = ref(props.dialogClass);
const _title = ref(props.title);
const bindParams = ref<{ footer?: string }>({});
if (props.showFooter === false) {
    bindParams.value.footer = "";
}

// params参数是可变的（比如相册里的添加子相册按钮），这里需要做监听
const params = computed(() => {
    return Object.assign(props.params, { isDialog: true });
});
const _width = computed(() => {
    if (props.type == "gallery") {
        return "811px";
    } else {
        return props.width;
    }
});

function show() {
    confirmLoading.value = false;
    loaded.value = true;
    visible.value = true;
}
function close(e: any) {
    visible.value = false;
}
const submitForm = ref();

const submitCallback = (result: any) => {
    visible.value = false;
    confirmLoading.value = false;
    // 执行父组件传来的确认回调，如刷新列表
    emit("okCallback", result, props.data);
};
const callback = (result: any) => {
    emit("callback", result, props.data);
};
//
const onOk = () => {
    // 执行子组件的表单提交方法，方法名称固定为onFormSubmit
    submitForm.value.onFormSubmit();
};

const okType = (type: boolean) => {
    okDisabled.value = !type;
};

defineExpose({
    show,
    close
});
</script>
<style lang="less" scoped>
.dialog-link {
    cursor: pointer;
}
</style>
