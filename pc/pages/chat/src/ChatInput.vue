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
                :before-upload="beforeMediaUpload"
                :http-request="handleMediaUpload"
                :disabled="videoUploading"
                accept=".jpg,.jpeg,.png,.gif,.bmp,.webp,.mp4,video/mp4"
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
/**
 * 聊天输入框组件
 * - 支持文本输入、表情选择、图片上传
 * - Enter发送消息，Shift+Enter换行
 * - 图片上传限制5MB，支持jpg/png/gif/bmp/webp
 */
import { ref } from "vue";
import { ElMessage } from "element-plus";
import type { UploadRequestOptions } from "element-plus";
import Emoji from "./Emoji.vue";
import { requestHeader, requestPrefix } from "~/utils/request";
const { t } = useI18n();

interface SendTarget {
    targetConversationId?: number;
    targetShopId: number;
}

interface SendPayload extends Partial<SendTarget> {
    type: string;
    content?: string;
    pic?: string;
    videoUrl?: string;
    videoCover?: string;
    pendingKey?: string;
}

interface VideoPendingPayload extends SendTarget {
    key: string;
    status: string;
    percent?: number;
    cover?: string;
    width?: number;
    height?: number;
}

const props = defineProps<{
    conversationId?: number;
    shopId: number;
}>();

const emit = defineEmits<{
    (e: "send", data: SendPayload): void;
    (e: "videoPending", payload: VideoPendingPayload): void;
}>();

const msgContent = ref("");
const showEmojiPicker = ref(false);
const textareaRef = ref<HTMLTextAreaElement | null>(null);

// 允许的图片类型
const allowedImageTypes = ["image/jpeg", "image/jpg", "image/png", "image/gif", "image/bmp", "image/webp"];

// 视频最大大小 (MB)
const videoMaxSizeMb = 50;

// 与会话气泡中视频的展示尺寸保持一致
const videoBubbleMaxWidth = 240;
const videoBubbleMaxHeight = 240;

const videoUploadingKeys = ref<string[]>([]);
const videoUploading = computed(() => videoUploadingKeys.value.length > 0);

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

const isVideoFile = (file: File) => {
    return file.type.startsWith("video/") || file.name.toLowerCase().endsWith(".mp4");
};

// 图片与视频共用同一个入口，按文件类型分派到各自的校验与上传逻辑
const beforeMediaUpload = (file: File) => {
    return isVideoFile(file) ? beforeVideoUpload(file) : beforeImageUpload(file);
};

const handleMediaUpload = (options: UploadRequestOptions) => {
    return isVideoFile(options.file as File) ? handleVideoUpload(options) : handleImageUpload(options);
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
    const targetConversationId = props.conversationId;
    const targetShopId = props.shopId;

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
                pic: result.data.picUrl,
                targetConversationId,
                targetShopId
            });
        } else {
            ElMessage.error(result.message || t("图片上传失败"));
        }
    } catch (error) {
        console.error("图片上传失败:", error);
        ElMessage.error(t("图片上传失败，请重试"));
    }
};

// 视频上传前验证
const beforeVideoUpload = (file: File) => {
    const isMp4 = file.type === "video/mp4" || file.name.toLowerCase().endsWith(".mp4");
    if (!isMp4) {
        ElMessage.error(t("只能上传mp4格式的视频"));
        return false;
    }
    if (file.size > videoMaxSizeMb * 1024 * 1024) {
        ElMessage.error(t("视频大小不能超过{size}MB", { size: videoMaxSizeMb }));
        return false;
    }
    return true;
};

// 抽取本地首帧作为占位封面，并按气泡展示尺寸换算宽高，避免发送成功后气泡尺寸跳变
const captureVideoPoster = (file: File): Promise<{ cover: string; width: number; height: number }> => {
    return new Promise((resolve) => {
        const objectUrl = URL.createObjectURL(file);
        const video = document.createElement("video");
        let settled = false;
        const finish = (cover: string, width: number, height: number) => {
            if (settled) return;
            settled = true;
            URL.revokeObjectURL(objectUrl);
            resolve({ cover, width, height });
        };
        video.muted = true;
        video.preload = "metadata";
        video.src = objectUrl;
        video.onloadeddata = () => {
            try {
                const canvas = document.createElement("canvas");
                canvas.width = video.videoWidth;
                canvas.height = video.videoHeight;
                const ctx = canvas.getContext("2d");
                if (!ctx || !canvas.width || !canvas.height) {
                    finish("", 0, 0);
                    return;
                }
                ctx.drawImage(video, 0, 0, canvas.width, canvas.height);
                const scale = Math.min(videoBubbleMaxWidth / canvas.width, videoBubbleMaxHeight / canvas.height, 1);
                finish(canvas.toDataURL("image/jpeg", 0.7), Math.round(canvas.width * scale), Math.round(canvas.height * scale));
            } catch (error) {
                finish("", 0, 0);
            }
        };
        video.onerror = () => finish("", 0, 0);
        setTimeout(() => finish("", 0, 0), 3000);
    });
};

// 以 XHR 上传以获取真实上传进度
const uploadVideoWithProgress = (file: File, onProgress: (percent: number) => void): Promise<any> => {
    return new Promise((resolve, reject) => {
        const formData = new FormData();
        formData.append("file", file);

        const xhr = new XMLHttpRequest();
        xhr.open("POST", requestPrefix + "/user/user/uploadVideo");

        const headers = requestHeader() as Record<string, any>;
        Object.keys(headers).forEach((name) => {
            if (headers[name] !== undefined && headers[name] !== null) {
                xhr.setRequestHeader(name, String(headers[name]));
            }
        });

        xhr.upload.onprogress = (event) => {
            if (event.lengthComputable) {
                onProgress(Math.round((event.loaded / event.total) * 100));
            }
        };
        xhr.onload = () => {
            try {
                resolve(JSON.parse(xhr.responseText));
            } catch (error) {
                reject(error);
            }
        };
        xhr.onerror = () => reject(new Error("upload failed"));
        xhr.send(formData);
    });
};

// 自定义上传视频
const handleVideoUpload = async (options: UploadRequestOptions) => {
    const file = options.file as File;
    const key = `${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    const targetConversationId = props.conversationId;
    const targetShopId = props.shopId;
    const sendTarget = { targetConversationId, targetShopId };

    videoUploadingKeys.value.push(key);
    emit("videoPending", { key, status: "start", ...sendTarget });
    captureVideoPoster(file).then(({ cover, width, height }) => {
        if (cover) {
            emit("videoPending", { key, status: "cover", cover, width, height, ...sendTarget });
        }
    });

    try {
        const result = await uploadVideoWithProgress(file, (percent) => {
            // percent 只统计字节推送，服务端接收处理和消息发送不在其中，压到 95% 以内留给后续阶段
            if (percent >= 100) {
                emit("videoPending", { key, status: "processing", ...sendTarget });
            } else {
                emit("videoPending", { key, status: "progress", percent: Math.min(95, Math.round(percent * 0.95)), ...sendTarget });
            }
        });

        if (result.code === 0 && result.data?.videoUrl) {
            emit("videoPending", { key, status: "processing", ...sendTarget });
            emit("send", {
                type: "video",
                videoUrl: result.data.videoUrl,
                videoCover: result.data.videoCover,
                pendingKey: key,
                ...sendTarget
            });
        } else {
            ElMessage.error(result.message || t("视频上传失败"));
            emit("videoPending", { key, status: "fail", ...sendTarget });
        }
    } catch (error) {
        console.error("视频上传失败:", error);
        ElMessage.error(t("视频上传失败，请重试"));
        emit("videoPending", { key, status: "fail", ...sendTarget });
    } finally {
        videoUploadingKeys.value = videoUploadingKeys.value.filter((item) => item !== key);
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
            // 保持输入内容原样大小写（全局 html 有 text-transform: capitalize）
            text-transform: none;

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
