<template>
    <el-upload ref="upload" v-model:file-list="fileList" :action="requestPrefix + '/' + apiUrl"
        :headers="uploadHeaders" :before-upload="beforeAvatarUpload" :on-exceed="handleExceed"
        :on-preview="handlePictureCardPreview" :on-remove="handleRemove" :on-success="handleSuccess"
        :on-progress="handleProgress" :on-error="handleError" :limit="limit" :accept="pickerAccept"
        :multiple="limit > 1 ? true : false" list-type="picture-card"
        :class="{ 'hide-upload-button': limit == 1 && fileList.length > 0 }">
        <!-- 上传按钮 -->
        <template #trigger v-if="fileList.length === 0">
            <el-icon>
                <Plus />
            </el-icon>
        </template>
    </el-upload>
    <el-dialog v-model="dialogVisible">
        <div class="image">
            <el-image :src="dialogImageUrl" alt="" fit="cover" />
        </div>
    </el-dialog>
</template>
<script lang="ts" setup>
import { computed, nextTick, ref } from "vue";
import { Plus } from "@element-plus/icons-vue";
import { requestPrefix, requestHeader } from "~/utils/request";
import { useFormItem } from "element-plus";
import type { UploadProps, UploadUserFile } from "element-plus";
import { imageFormat } from "~/utils/format";
const uploadHeaders = requestHeader();
const { t } = useI18n();
const upload = ref<any>();
const props = defineProps({
    limit: {
        type: Number,
        default: 1
    },
    fileTypes: {
        type: Array,
        default: []
    },
    apiUrl: {
        type: String,
        default: "user/user/uploadImg"
    },
    responseDataType: {
        type: String,
        default: "gallery"
    },
    // 文件选择器可选类型，如 ".jpg,.png,.pdf"
    accept: {
        type: String,
        default: ""
    },
    // 单个文件大小上限（MB）
    maxSize: {
        type: Number,
        default: 2
    }
});
const emit = defineEmits(["uploading-change"]);
// 上传列表变化不会经过原生表单控件，需主动通知所在 el-form-item 重新校验，否则必填提示不会消失
const { formItem } = useFormItem();
const validateFormItem = () => {
    nextTick(() => formItem?.validate("change").catch(() => {}));
};
// 去重后的允许类型（调用方存在 ['jpeg', 'png', 'jpeg'] 这类重复写法）
const allowedTypes = computed(() => [...new Set(props.fileTypes as string[])]);
// 未传 accept 时按 fileTypes 生成扩展名过滤，避免文件选择框默认可选任意文件
const pickerAccept = computed(() => {
    if (props.accept) return props.accept;
    return allowedTypes.value.flatMap((type) => (type === "jpeg" ? [".jpg", ".jpeg"] : [`.${type}`])).join(",");
});
const uploading = ref(false);
const beforeAvatarUpload: UploadProps["beforeUpload"] = (rawFile) => {
    // 检查是否已经达到上传限制
    if (fileList.value.length >= props.limit) {
        ElMessage.error(t("已经达到上传数量限制"));
        return false; // 拒绝这次上传
    }
    const fileName = rawFile.type.replace(/.*\//, "");
    // 部分系统选择 PDF 等文件时 MIME 为空，按扩展名兜底（jpg 与 MIME 子类型 jpeg 对齐）
    const extension = (rawFile.name.split(".").pop() || "").toLowerCase().replace(/^jpg$/, "jpeg");
    if (!allowedTypes.value.includes(fileName) && !allowedTypes.value.includes(extension)) {
        ElMessage.error(`${t("无法上传")} ${fileName || extension} ${t("文件!")}` + `${t("只能上传")} ${allowedTypes.value.join(", ")} ${t("文件!")}`);
        return false;
    }
    if (rawFile.size / 1024 / 1024 > props.maxSize) {
        ElMessage.error(props.maxSize === 2 ? t("只能上传2M以内的图片") : `${t("文件大小不能超过")}${props.maxSize}MB`);
        return false;
    }
    return true;
};
const uploadList: any = defineModel("uploadList", { default: [] });
const fileList = ref<UploadUserFile[]>([]);
const processingData = () => {
    if(uploadList.value == null){
        uploadList.value = [];
        return
    }
    if (uploadList.value.length > 0) {
        uploadList.value.forEach((item: any) => {
            item.url = imageFormat(item.picUrl);
        });
        fileList.value = uploadList.value;
    }
};
processingData();

const dialogImageUrl = ref("");
const dialogVisible = ref(false);

const handleRemove: UploadProps["onRemove"] = (uploadFile: any, uploadFiles) => {
    if (uploadFile && uploadList.value) {
        const picThumb = uploadFile.picThumb || uploadFile.response?.data?.picThumb || uploadFile.response?.data;
        uploadList.value = uploadList.value.filter((item: any) => item.picThumb !== picThumb);
    } else {
        uploadList.value = [];
    }
    validateFormItem();
};
const handleExceed: UploadProps["onExceed"] = (files: File[] = [], uploadFiles: UploadUserFile[]) => {
    if (props.limit === 1 && fileList.value.length > 0) {
        fileList.value.pop();
        uploadList.value.pop();
        upload.value.clearFiles(); //删除所有上传的文件
        upload.value.handleStart(files[0]); //handleStart()指的是手动选择文件，Element Plus 的el-upload有说明
        upload.value.submit(); //手动上传
    }
};

const handlePictureCardPreview: UploadProps["onPreview"] = (uploadFile) => {
    dialogImageUrl.value = uploadFile.url!;
    dialogVisible.value = true;
};
const handleSuccess: UploadProps["onSuccess"] = (response, uploadFile) => {
    uploading.value = false;
    emit("uploading-change", false);
    if (response?.code !== 0 || !response?.data) {
        fileList.value = fileList.value.filter((item) => item.uid !== uploadFile.uid);
        ElMessage.error(response?.message || t("图片上传失败"));
        return;
    }
    try {
        if (props.responseDataType === "url") {
            const url = response.data;
            uploadList.value.push({
                picThumb: url,
                picUrl: url,
                picName: uploadFile.name,
                url
            });
            uploadFile.url = imageFormat(url);
            validateFormItem();
            return;
        }
        uploadList.value.push({
            picThumb: response.data.picThumb,
            picUrl: response.data.picUrl,
            picId: response.data.picId,
            picName: response.data.picName
        });
        validateFormItem();
    } catch (e) { }
};
const handleProgress: UploadProps["onProgress"] = () => {
    if (uploading.value) return;
    uploading.value = true;
    emit("uploading-change", true);
};
const handleError: UploadProps["onError"] = () => {
    uploading.value = false;
    emit("uploading-change", false);
};
</script>
<style lang="less" scoped>
.hide-upload-button {
    :deep(.el-upload--picture-card) {
        display: none !important;
    }
}

:deep(.el-upload-list--picture-card .el-upload-list__item) {
    width: 80px;
    /* 设置缩略图的宽度 */
    height: 80px;
    /* 设置缩略图的高度 */
}

:deep(.el-upload--picture-card) {
    width: 80px;
    /* 设置缩略图的宽度 */
    height: 80px;
    /* 设置缩略图的高度 */
}

:deep(.el-upload-list__item-status-label) {
    display: none !important;
}

.image {
    width: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
}
</style>
