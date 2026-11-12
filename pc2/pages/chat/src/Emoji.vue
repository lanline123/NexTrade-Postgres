<template>
    <div class="emoji-picker">
        <div class="emoji-list">
            <div
                v-for="item in emojiList"
                :key="item.id"
                class="emoji-item"
                @click="insertEmoji(item.name)"
                :title="item.name"
            >
                <img :src="`/assets/emoji/${item.id}.png`" :alt="item.name" class="emoji-img" />
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
import emojiElements from "./emojiElements";

const emit = defineEmits(["insertEmoji"]);

const emojiList = emojiElements;

const insertEmoji = (emoji: string) => {
    emit("insertEmoji", emoji);
};
</script>

<style lang="less" scoped>
.emoji-picker {
    width: 100%;
    max-height: 200px;
    overflow-y: auto;

    &::-webkit-scrollbar {
        width: 6px;
    }
    &::-webkit-scrollbar-thumb {
        background: #e0e0e0;
        border-radius: 3px;
    }

    .emoji-list {
        display: grid;
        grid-template-columns: repeat(10, 1fr);
        gap: 8px;
        padding: 10px;

        .emoji-item {
            width: 32px;
            height: 32px;
            display: flex;
            align-items: center;
            justify-content: center;
            cursor: pointer;
            border-radius: 4px;
            transition: all 0.2s;

            &:hover {
                background-color: #f0f0f0;
                transform: scale(1.2);
            }

            .emoji-img {
                width: 24px;
                height: 24px;
                display: block;
            }
        }
    }
}
</style>

