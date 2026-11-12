<template>
    <div
        ref="panelRef"
        class="image-search-panel"
        tabindex="-1"
        role="dialog"
        :aria-label="$t('上传图片搜索')"
        @paste="onPaste"
        @keydown.esc.stop.prevent="emit('close')"
    >
        <div class="image-search-panel__header">
            <span class="image-search-panel__title">{{ $t("上传图片搜索") }}</span>
            <button type="button" class="image-search-panel__close" :aria-label="$t('关闭')" @click="emit('close')">×</button>
        </div>

        <!-- 待选图 -->
        <template v-if="status === 'idle'">
            <div
                class="image-search-panel__drop"
                :class="{ 'image-search-panel__drop--active': dragging }"
                @dragenter.prevent="dragging = true"
                @dragover.prevent="dragging = true"
                @dragleave.prevent="dragging = false"
                @drop.prevent="onDrop"
            >
                <svg class="image-search-panel__drop-icon" viewBox="0 0 48 48" fill="none" aria-hidden="true">
                    <rect x="5" y="7" width="38" height="34" rx="6" stroke="currentColor" stroke-width="3"></rect>
                    <circle cx="15" cy="17" r="2.5" fill="currentColor"></circle>
                    <path d="M6 31c7-7 12-7 17-2s10 3 19-6" stroke="currentColor" stroke-width="3" stroke-linecap="round"></path>
                </svg>
                <p class="image-search-panel__drop-text">{{ $t("{0}粘贴图片到此处", [pasteShortcut]) }}</p>
            </div>
            <p v-if="errorMessage" class="image-search-panel__error" role="alert">{{ errorMessage }}</p>
            <div class="image-search-panel__actions">
                <button type="button" class="image-search-panel__btn image-search-panel__btn--primary" @click="pickFile">{{ $t("上传图片") }}</button>
            </div>
        </template>

        <!-- 预览 -->
        <template v-else>
            <div class="image-search-panel__preview">
                <img v-if="previewUrl" :src="previewUrl" :alt="$t('待搜索的图片')" class="image-search-panel__image" @error="onPreviewError" />
                <div v-else class="image-search-panel__file">
                    <CameraIcon class="image-search-panel__drop-icon" />
                    <span class="image-search-panel__file-name">{{ file?.name }}</span>
                </div>
            </div>
            <p v-if="errorMessage" class="image-search-panel__error" role="alert">{{ errorMessage }}</p>
            <div class="image-search-panel__actions">
                <button ref="submitRef" type="button" class="image-search-panel__btn image-search-panel__btn--primary" @click="submit">
                    {{ $t("搜索") }}
                </button>
            </div>
        </template>

        <input ref="inputRef" type="file" class="image-search-panel__input" :accept="imageSearchStore.accept" tabindex="-1" @change="onInputChange" />
    </div>
</template>

<script setup lang="ts">
/**
 * 以图搜图上传面板
 * 选图、拖拽、粘贴后只在本地预览，不发起检索；点击搜索时把图片交给结果页上传检索
 */
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef } from "vue";
import CameraIcon from "~/components/icons/CameraIcon.vue";
import { useImageSearchStore } from "~/store/imageSearch";

type PanelStatus = "idle" | "preview";

const emit = defineEmits<{
    (e: "close"): void;
    (e: "submit", payload: { file: File }): void;
}>();

const { t } = useI18n();
const imageSearchStore = useImageSearchStore();

const panelRef = ref<HTMLElement | null>(null);
const inputRef = ref<HTMLInputElement | null>(null);
const submitRef = ref<HTMLButtonElement | null>(null);
const status = ref<PanelStatus>("idle");
const file = shallowRef<File | null>(null);
const previewUrl = ref("");
const errorMessage = ref("");
const dragging = ref(false);
const pasteShortcut = computed(() => (/Mac|iPhone|iPad/i.test(navigator.platform || navigator.userAgent) ? "⌘ + V" : "Ctrl + V"));
const formatText = computed(() => imageSearchStore.formats.join(" / "));

// ===================== 图片选择 =====================
const releasePreview = (): void => {
    if (previewUrl.value) URL.revokeObjectURL(previewUrl.value);
    previewUrl.value = "";
};

/** 校验并接收一批候选文件；一次只能使用一张，不静默取第一张 */
const acceptFiles = (files: File[]): void => {
    if (files.length === 0) return;
    if (files.length > 1) {
        errorMessage.value = t("一次只能使用一张图片");
        return;
    }
    const next = files[0];
    if (!imageSearchStore.isSupportedFile(next)) {
        errorMessage.value = formatText.value ? t("图片格式不支持，请使用 {0} 格式", [formatText.value]) : t("请选择图片文件");
        return;
    }
    if (next.size === 0) {
        errorMessage.value = t("无法读取图片，请更换图片");
        return;
    }
    releasePreview();
    file.value = next;
    previewUrl.value = imageSearchStore.canPreview(next) ? URL.createObjectURL(next) : "";
    errorMessage.value = "";
    status.value = "preview";
    nextTick(() => submitRef.value?.focus());
};

const pickFile = (): void => {
    inputRef.value?.click();
};

const onInputChange = (event: Event): void => {
    const input = event.target as HTMLInputElement;
    acceptFiles(Array.from(input.files ?? []));
    input.value = "";
};

const onDrop = (event: DragEvent): void => {
    dragging.value = false;
    const files = Array.from(event.dataTransfer?.files ?? []);
    if (files.length === 0) return;
    const images = files.filter((item) => item.type.startsWith("image/") || imageSearchStore.isSupportedFile(item));
    if (images.length === 0) {
        errorMessage.value = t("请选择图片文件");
        return;
    }
    acceptFiles(images);
};

/** 只处理面板内用户主动触发的图片粘贴，没有图片的粘贴不拦截 */
const onPaste = (event: ClipboardEvent): void => {
    const images = Array.from(event.clipboardData?.files ?? []).filter((item) => item.type.startsWith("image/"));
    if (images.length === 0) return;
    event.preventDefault();
    acceptFiles(images);
};

/** 图片无法解码时回到待选图，提示重新选择 */
const onPreviewError = (): void => {
    releasePreview();
    file.value = null;
    status.value = "idle";
    errorMessage.value = t("无法读取图片，请更换图片");
};

// ===================== 搜索 =====================
const submit = (): void => {
    if (file.value) emit("submit", { file: file.value });
};

// ===================== 生命周期 =====================
onMounted(() => {
    panelRef.value?.focus();
});

onBeforeUnmount(() => {
    releasePreview();
});
</script>

<style scoped lang="less">
.image-search-panel {
    position: relative;
    box-sizing: border-box;
    width: 100%;
    padding: 16px;
    color: #1a1a1a;
    font-size: 14px;
    line-height: 1.5;
    text-align: left;
    outline: none;

    &__header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 12px;
    }

    &__title {
        font-size: 14px;
        font-weight: 600;
        line-height: 22px;
    }

    &__close {
        width: 22px;
        height: 22px;
        margin-right: -4px;
        padding: 0;
        border: 0;
        border-radius: 4px;
        background: transparent;
        color: #1a1a1a;
        font-size: 22px;
        font-weight: 300;
        line-height: 20px;
        cursor: pointer;

        &:hover,
        &:focus-visible {
            color: var(--general);
        }
    }

    &__drop,
    &__preview {
        position: relative;
        display: flex;
        align-items: center;
        justify-content: center;
        box-sizing: border-box;
        height: 205px;
        border: 1px solid #e8e8e8;
        border-radius: 6px;
        background: #f8f8f9;
        overflow: hidden;
    }

    &__drop {
        flex-direction: column;
        gap: 8px;
        padding: 0 8px;
        text-align: center;
        transition:
            border-color 0.2s,
            background-color 0.2s;

        &--active {
            border-color: var(--general);
            background: #fff;
        }
    }

    &__drop-icon {
        width: 32px;
        height: 32px;
        color: #c9c9c9;
    }

    &__drop-text {
        margin: 0;
        color: #b3b3b3;
        font-size: 14px;
        line-height: 22px;
    }

    &__image {
        max-width: 100%;
        max-height: 100%;
        object-fit: contain;
    }

    &__file {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 8px;
        max-width: 80%;
        color: #8c8c8c;
    }

    &__file-name {
        max-width: 100%;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    &__error {
        margin: 8px 0 0;
        color: #f5222d;
        font-size: 12px;
        word-break: break-word;
    }

    &__actions {
        display: flex;
        gap: 8px;
        margin-top: 12px;
    }

    &__btn {
        flex: 1;
        min-width: 0;
        height: 39px;
        padding: 0 8px;
        border: 1px solid #dcdcdc;
        border-radius: 6px;
        background: #fff;
        color: #1a1a1a;
        font-size: 12px;
        font-weight: 600;
        cursor: pointer;

        &:hover,
        &:focus-visible {
            border-color: var(--general);
            color: var(--general);
        }

        &--primary {
            border-color: var(--general);
            background: var(--general);
            color: var(--main-text, #fff);

            &:hover,
            &:focus-visible {
                color: var(--main-text, #fff);
                opacity: 0.88;
            }
        }
    }

    &__input {
        display: none;
    }
}
</style>
