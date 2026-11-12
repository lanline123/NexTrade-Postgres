<template>
    <span v-html="parsedHtml" class="emoji-content"></span>
</template>

<script lang="ts" setup>
import { computed } from "vue";
import { replaceEmojiToHtml } from "./emojiElements";

const props = defineProps({
    content: {
        type: String,
        default: ""
    }
});

const parsedHtml = computed(() => {
    return replaceEmojiToHtml(props.content);
});
</script>

<style lang="less" scoped>
.emoji-content {
    :deep(.emoji) {
        width: 20px;
        height: 20px;
        vertical-align: middle;
        margin: 0 2px;
    }
}
</style>

