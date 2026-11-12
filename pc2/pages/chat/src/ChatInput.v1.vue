<!--
  o2o 原版组件存档（v1）
  切回方法：删除当前 ChatInput.vue，将本文件重命名为 ChatInput.vue
-->
<template>
    <div class="chat-input-container">
        <div class="toolbar">
            <div class="tool-btn emoji-btn" :title="$t('表情')" @click="toggleEmojiPicker">
                <svg
                    viewBox="0 0 24 24"
                    width="24"
                    height="24"
                    stroke="currentColor"
                    stroke-width="2"
                    fill="none"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                >
                    <circle cx="12" cy="12" r="10"></circle>
                    <path d="M8 14s1.5 2 4 2 4-2 4-2"></path>
                    <line x1="9" y1="9" x2="9.01" y2="9"></line>
                    <line x1="15" y1="9" x2="15.01" y2="9"></line>
                </svg>
            </div>
            <el-upload
                class="image-upload"
                :show-file-list="false"
                :before-upload="beforeImageUpload"
                :http-request="handleImageUpload"
                accept=".jpg,.jpeg,.png,.gif,.bmp,.webp"
            >
                <div class="tool-btn image-btn" :title="$t('图片')">
                    <svg
                        viewBox="0 0 24 24"
                        width="24"
                        height="24"
                        stroke="currentColor"
                        stroke-width="2"
                        fill="none"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                        <circle cx="8.5" cy="8.5" r="1.5"></circle>
                        <polyline points="21 15 16 10 5 21"></polyline>
                    </svg>
                </div>
            </el-upload>
        </div>

        <!-- 表情选择器 -->
        <div v-show="showEmojiPicker" class="emoji-picker-wrapper">
            <Emoji @insertEmoji="handleInsertEmoji" />
        </div>

        <div class="input-area">
            <textarea
                ref="textareaRef"
                class="msg-input"
                :placeholder="$t('请输入消息，按Enter键或 点击发送按钮发送')"
                v-model="msgContent"
                @keydown="handleKeydown"
                @focus="handleFocus"
            ></textarea>
        </div>

        <div class="action-area">
            <span class="tips">{{ $t('按 Shift + Enter 换行') }}</span>
            <button class="send-btn" :disabled="!msgContent.trim()" @click="sendMsg">{{ $t('发送') }}</button>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { ElMessage } from "element-plus";
import type { UploadRequestOptions } from "element-plus";
import Emoji from "./Emoji.vue";
import { requestHeader, requestPrefix } from "~/utils/request";
const { t } = useI18n();
const emit = defineEmits<{
    (e: "send", data: { type: string; content?: string; pic?: string }): void;
}>();

const msgContent = ref("");
const showEmojiPicker = ref(false);
const textareaRef = ref<HTMLTextAreaElement | null>(null);

// 允许的图片类型
const allowedImageTypes = ["image/jpeg", "image/jpg", "image/png", "image/gif", "image/bmp", "image/webp"];

// 切换表情选择器
const toggleEmojiPicker = () => {
    showEmojiPicker.value = !showEmojiPicker.value;
};

// 插入表情
const handleInsertEmoji = (emoji: string) => {
    msgContent.value += `[${emoji}]`;
    // 聚焦输入框
    textareaRef.value?.focus();
};

// 输入框获取焦点时关闭表情选择器
const handleFocus = () => {
    // 不自动关闭，让用户可以连续选择表情
};

// 发送消息
const sendMsg = () => {
    if (!msgContent.value.trim()) return;
    emit("send", {
        type: "text",
        content: msgContent.value
    });
    msgContent.value = "";
    showEmojiPicker.value = false;
};

// 处理键盘事件
const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
        if (!e.shiftKey) {
            e.preventDefault();
            sendMsg();
        }
    }
};

// 上传前验证
const beforeImageUpload = (file: File) => {
    // 验证文件类型
    if (!allowedImageTypes.includes(file.type)) {
        ElMessage.error(t("请选择图片文件（支持 jpg、png、gif、bmp、webp 格式）"));
        return false;
    }

    // 验证文件大小 (最大 5MB)
    if (file.size > 5 * 1024 * 1024) {
        ElMessage.error(t("图片大小不能超过5MB"));
        return false;
    }

    return true;
};

// 自定义上传图片
const handleImageUpload = async (options: UploadRequestOptions) => {
    const { file } = options;

    try {
        // 上传图片
        const formData = new FormData();
        formData.append("file", file);

        const response = await fetch(requestPrefix + "/user/user/uploadImg", {
            method: "POST",
            headers: {
                ...requestHeader()
            },
            body: formData
        });

        const result = await response.json();

        if (result.code === 0 && result.data?.picUrl) {
            emit("send", {
                type: "image",
                pic: result.data.picUrl
            });
        } else {
            ElMessage.error(result.message || t("图片上传失败"));
        }
    } catch (error) {
        console.error("图片上传失败:", error);
        ElMessage.error(t("图片上传失败，请重试"));
    }
};

// 暴露方法
defineExpose({
    focus: () => textareaRef.value?.focus()
});
</script>

<style lang="less" scoped>
.chat-input-container {
    min-height: 160px;
    background-color: #f7f8fc;
    border-top: 1px solid #e7e7e7;
    display: flex;
    flex-direction: column;
    padding: 10px 0;
    //margin: 0 20px;
    position: relative;

    .toolbar {
        display: flex;
        gap: 15px;
        margin-bottom: 8px;

        .tool-btn {
            cursor: pointer;
            color: #2a2a2a;
            transition: color 0.2s;
            padding: 4px;
            border-radius: 4px;

            &:hover {
                color: #1890ff;
                background-color: #f5f5f5;
            }
        }

        .image-upload {
            line-height: 1;

            :deep(.el-upload) {
                line-height: 1;
            }
        }
    }

    .emoji-picker-wrapper {
        position: absolute;
        bottom: 100%;
        left: 20px;
        background-color: #fff;
        border: 1px solid #e0e0e0;
        border-radius: 8px;
        box-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
        z-index: 100;
        width: 420px;
    }

    .input-area {
        flex: 1;
        margin-bottom: 10px;

        .msg-input {
            width: 100%;
            height: 100%;
            min-height: 60px;
            border: none;
            background: transparent;
            resize: none;
            outline: none;
            font-size: 14px;
            line-height: 1.5;
            color: #333;
            padding: 0 12px;
            box-sizing: border-box;

            &::placeholder {
                color: #ccc;
            }
        }
    }

    .action-area {
        display: flex;
        justify-content: flex-end;
        align-items: center;
        gap: 15px;
        margin-right: 12px;

        .tips {
            font-size: 12px;
            color: #999;
        }

        .send-btn {
            background-color: var(--general);
            color: white;
            border: none;
            //padding: 8px 24px;
            border-radius: 4px;
            cursor: pointer;
            font-size: 12px;
            transition: all 0.2s;
            width: 50px;
            height: 28px;

            &:hover:not(:disabled) {
                //background: var(--primary-light-9);
            }

            &:disabled {
                background-color: #d9d9d9;
                cursor: not-allowed;
            }
        }
    }
}
</style>
