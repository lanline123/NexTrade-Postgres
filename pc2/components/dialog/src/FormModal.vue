<template>
    <MyModal
        v-model:open="visible"
        :confirm-loading="confirmLoading"
        :width="width"
        :okText="$t('确认')"
        :cancelText="$t('取消')"
        :destroyOnClose="true"
        @ok="onOk"
        :ok-button-props="{ disabled: okDisabled }"
        :footer-style="{ textAlign: 'right' }"
        :title="title"
        :bodyStyle="bodyStyle"
    >
        <a-config-provider :locale="zhCN">
            <Component
                ref="submitForm"
                @submitCallback="submitCallback"
                @show="show"
                @close="close"
                @okType="okType"
                v-bind="params"
                v-model:confirmLoading="confirmLoading"
            >
            </Component>
        </a-config-provider>
        <template #footer>
            <a-button v-if="showFooter" style="margin-right: 8px" @click="close">{{ $t("取消") }}</a-button>
            <a-button v-if="showFooter" :disabled="okDisabled" :loading="confirmLoading" type="primary" @click="onOk">{{ $t("确认") }}</a-button>
        </template>
    </MyModal>
</template>
<script setup lang="ts">
import { ref, defineAsyncComponent, h, toRaw, Comment } from "vue";
import zhCN from "ant-design-vue/es/locale/zh_CN";
import { Modal } from "ant-design-vue";
import { Drawer } from "ant-design-vue";

const props = defineProps({
    path: String, //路径
    params: {
        type: Object,
        default: () => ({})
    }, //绑定给页面的参数
    okCallback: Function,
    isDrawer: {
        type: Boolean,
        default: false
    },
    title: {
        type: String,
        default: ""
    },
    showFooter: {
        type: Boolean,
        default: true
    },
    width: {
        type: String,
        default: "800px"
    },
    maskClosable: {
        type: Boolean,
        default: true
    },
    closable: {
        type: Boolean,
        default: true
    },
    bodyStyle: {
        type: Object,
        default: () => ({})
    }
});
const MyModal = props.isDrawer == true ? Drawer : Modal;

// 加载多级目录，其中**代表多级
// const modules = import.meta.glob('../../../pages/**/*.vue');
const filePath = `../../../pages/${props.path}.vue`;
// 异步加载动态组件
const Component = defineAsyncComponent(()=>import(filePath));

const visible = ref<boolean>(false);
const confirmLoading = ref<boolean>(false);
const okDisabled = ref<boolean>(false);
const params = Object.assign(props.params, { isDialog: true });
function show(e) {
    visible.value = true;
}
function close(e) {
    visible.value = false;
}
const submitForm = ref();

const submitCallback = (result: any) => {
    if (!result.errcode) {
        // 如果返回结果无报错，关闭弹窗
        visible.value = false;
        // 执行父组件传来的确认回调，如刷新列表
        props.okCallback(result);
    }
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
