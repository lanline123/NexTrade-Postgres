<template>
    <div style="background-color: #fff">
        <CommonHeader title="招商计划"></CommonHeader>
        <JoinHeader></JoinHeader>
        <div class="bg-info">
            <div class="entry-main">
                <img class="bg-image" src="@/assets/images/join/ruzhubg.jpg" />
                <div class="module-part" :class="['part-' + index, 'size-' + item.size]" v-for="(item, index) in joinStore.module4">
                    <h3 class="part-title" :class="['title-' + item.align]">{{ item.title }}</h3>
                    <span class="content" v-html="item.content"></span>
                </div>
            </div>
        </div>
    </div>
    <CommonPageFooter :mt30="false"></CommonPageFooter>
</template>
<script lang="ts" setup>
import { ref } from "vue";
import JoinHeader from "~/pages/join/joinHeader.vue";
import { useJoinStore } from "~/store/join";
import { getMobileNav } from "~/api/home/home";
const { t } = useI18n();

const joinStore = useJoinStore();
const getBackgroundPicture = async () => {
    try {
        const result = await getMobileNav({ decorateSn: "settlingProcess" });
        Object.assign(joinStore.module4, result?.data);
    } catch (error) {
        console.log(error);
    } finally {
        joinStore.module4Loaded = true;
    }
};
if (joinStore.module4Loaded === false) {
    getBackgroundPicture();
}
</script>
<style lang="less" scoped>
.bg-info {
    width: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    .entry-main {
        position: relative;
        min-width: 1200px;

        .bg-image {
            width: 100%;
        }

        .module-part {
            position: absolute;
            background-color: transparent;
            display: flex;
            flex-direction: column;
            gap: 15px;

            .part-title {
                color: #3d97f7;
                font-weight: bold;
                font-size: 20px;
                box-sizing: border-box;
            }

            .title-1 {
                padding-left: 50px;
                text-align: left;
            }

            .title-2 {
                text-align: left;
            }

            .content {
                overflow-y: auto;
                font-size: 13px;
                white-space: pre-wrap; /* 保留空格和换行符，允许文本换行 */
                word-wrap: break-word; /* 允许长单词和 URL 断开换行 */
                word-break: break-all; /* 强制单词在任意位置断开换行 */
                line-height: 24px;

                &::-webkit-scrollbar {
                    display: none;
                }
            }
        }

        .size-0 {
            width: 260px;
            height: 145px;
        }

        .size-1 {
            width: 345px;
            height: 145px;
        }

        .size-2 {
            width: 367px;
            height: 255px;
        }

        .part-0 {
            left: 378px;
            top: 275px;
        }

        .part-1 {
            left: 457px;
            top: 611px;
        }

        .part-2 {
            left: 500px;
            top: 1230px;
        }

        .part-3 {
            left: 471px;
            top: 1625px;
        }

        .part-4 {
            left: 500px;
            top: 2047px;
        }

        .part-5 {
            left: 360px;
            top: 2560px;
        }

        .part-6 {
            left: 560px;
            top: 3074px;
        }

        .part-7 {
            left: 300px;
            top: 3672px;
        }

        .part-8 {
            left: 579px;
            top: 4125px;
        }
    }
}
</style>
