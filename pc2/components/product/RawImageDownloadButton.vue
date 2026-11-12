<template>
    <button v-if="download?.visible.value" class="raw-image-download" :class="{ 'raw-image-download--inline': inline }" type="button"
        :disabled="download.busy.value" :aria-busy="download.busy.value" :title="$t('下载商品原图压缩包')" @click="download.download()">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path d="M12 3v12m-5-5 5 5 5-5M3 16v5h18v-5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
        <span>{{ $t(download.busy.value ? "正在准备素材" : "素材下载") }}</span>
    </button>
</template>
<script setup lang="ts">
import { inject } from "vue";
import { rawImageDownloadKey } from "~/composables/useProductRawImageDownload";
defineProps<{ inline?: boolean }>();
const download = inject(rawImageDownloadKey, null);
</script>
<style scoped>
.raw-image-download {
    display: inline-flex;
    flex-shrink: 0;
    flex-direction: column;
    align-self: center;
    align-items: center;
    justify-content: center;
    gap: 2px;
    min-width: 48px;
    height: 40px;
    padding: 0 4px;
    border: 0;
    background: transparent;
    color: #666;
    cursor: pointer;
    font: inherit;
    font-size: 12px;
    font-weight: 400;
    line-height: 1;
    white-space: nowrap;
}
.raw-image-download svg { display: block; flex-shrink: 0; }
.raw-image-download--inline {
    flex-direction: row;
    gap: 6px;
    min-width: 0;
    height: auto;
    padding: 4px 0;
    color: inherit;
    font-size: inherit;
}
.raw-image-download--inline svg { width: 16px; height: 16px; }
.raw-image-download:hover { color: var(--general); }
.raw-image-download:disabled { cursor: wait; opacity: .6; }
.raw-image-download:focus-visible { outline: 2px solid currentColor; outline-offset: 2px; }
</style>
