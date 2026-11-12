<template>
    <a class="btn-link" @click="delRecord">
        <slot></slot>
    </a>
</template>
<script lang="ts" setup>
import { message } from "ant-design-vue";

const props = defineProps({
    requestApi: {
        type: Function,
        default: () => {}
    },
    params: {
        type: Object,
        default: {} as any
    },
    text: {
        type: String,
        default: "您确认要删除该数据吗"
    }
});
const { t } = useI18n();
const emit = defineEmits(["afterDelete"]);
//  删除项
const delRecord = () => {
    ElMessageBox.confirm(t(props.text), t("提示"), {
        confirmButtonText: t("确认"),
        cancelButtonText: t("取消"),
        type: "warning"
    })
        .then(() => {
            props.requestApi(props.params).then((result: any) => {
                message.success(t("删除成功"));
                emit("afterDelete", result);
            }).catch((error:any) => {
            message.error(error.message);
        });
        });
};
</script>
