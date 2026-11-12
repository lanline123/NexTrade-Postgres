<template>
    <div @click="show" :class="'dialog-link ' + props.class" :style="props.style">
        <slot></slot>
    </div>
    <MyModal
        v-if="loaded"
        :class="dialogClass"
        v-model:open="visible"
        :confirm-loading="confirmLoading"
        :width="_width"
        :title="_title"
        :okText="t('确认')"
        :cancelText="t('取消')"
        :destroyOnClose="true"
        @ok="onOk"
        :ok-button-props="{ disabled: okDisabled }"
        :footer-style="{ textAlign: 'right' }"
        v-bind="bindParams"
        :maskClosable="maskClosable"
        :closable="closable"
        centered
    >
        <a-config-provider :locale="zhCN">
            <MemberEditRegion
                ref="submitForm"
                @submitCallback="submitCallback"
                @callback="callback"
                @show="show"
                @close="close"
                @okType="okType"
                v-bind="params"
                v-model:confirmLoading="confirmLoading"
            >
            </MemberEditRegion>
        </a-config-provider>
        <template v-if="showClose || showOnOk" #footer>
            <a-button v-show="showClose" style="margin-right: 12px" @click="close">{{ $t("取消") }}</a-button>
            <a-button v-show="showOnOk" :disabled="okDisabled" :loading="confirmLoading" type="primary" @click="onOk">{{ $t("确认") }}</a-button>
        </template>
    </MyModal>
</template>
<script setup lang="ts">
import { ref, defineAsyncComponent, shallowRef, computed, onMounted } from "vue";
import zhCN from "ant-design-vue/es/locale/zh_CN";

import { Drawer, Modal } from "ant-design-vue";
const { t } = useI18n();
const loaded = ref(false);
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
const MyModal = props.isDrawer == true ? Drawer : Modal;
const visible = ref<boolean>(false);
const confirmLoading = ref<boolean>(false);
const okDisabled = ref<boolean>(false);
const dialogClass = ref(props.dialogClass);
const _title = ref(props.title);
const bindParams = ref<{ footer?: string }>({});
if (props.showFooter === false) {
    bindParams.value.footer = null;
}
const path = ref(props.path);
// const componentPath = 'member/EditRegion' // 变量组件路径
// const dynamicComponent = ref(null)

// // 使用 import() 方法动态导入组件

// // 使用 defineAsyncComponent 方法动态定义异步组件
// const asyncComponent = defineAsyncComponent(() => import(`@/components/${componentPath}.vue`))

// // 将异步组件实例赋值给 dynamicComponent 变量
// dynamicComponent.value = asyncComponent

let _path = props.path;
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
// 相册弹窗
if (props.type == "gallery") {
    dialogClass.value += " " + "lyecs-modal-gallery noPadding";
    _path = "gallery/Gallery";
    _title.value = t("相册");
}
function show(e) {
    if (loaded.value == false) {
        loadComponent();
    }
    confirmLoading.value = false;
    loaded.value = true;
    visible.value = true;
}
function close(e) {
    visible.value = false;
}
const submitForm = ref();

async function loadComponent() {
    // const resolvedPath = `@/components/${path}`;
    // const componentModule = await import(resolvedPath);
    // dynamicComponent.value = resolveComponent(path)
    // const [directory, filename] = path.split('/')
    // const fullPath = `@/components/member/EditRegion.vue`
    // import(`@/components/member/EditRegion.vue`).then(module => {
    //     dynamicComponent.value = module.default
    // })
    // import(`@/components/${path}.vue`).then(module => {
    //     dynamicComponent.value = module.default
    // })
    // return new Promise((resolve, reject) => {
    //     // 加载多级目录，其中**代表多级
    //     const components = import.meta.glob("../../../views/**/*.vue");
    //     // 异步加载动态组件
    //     Component.value = defineAsyncComponent(components['../../../pages/' + _path + '.vue'])
    // });
}

const submitCallback = (result: any) => {
    if (!result.errcode) {
        // 如果返回结果无报错，关闭弹窗
        visible.value = false;
        confirmLoading.value = false;
        // 执行父组件传来的确认回调，如刷新列表
        emit("okCallback", result, props.data);
    }
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
