<template>
    <div class="simple-catbox">
        <div class="cat-wrap">
            <template v-for="item in productList">
                <NuxtLink :to="urlFormat({ path: 'list', seoLink: item.seoLink, id: item.categoryId })">
                    <div class="item">
                        <div class="pic">
                            <img :src="imageFormat(item.categoryPic)" :alt="item.categoryName || ''" />
                        </div>
                        <div class="tit">{{ item.categoryName }}</div>
                    </div>
                </NuxtLink>
            </template>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref } from "vue";
import { getHomeRecommendProductData } from "~/api/home/home";
import { useHomeStore } from "~/store/home";
const homeStore = useHomeStore();
const props = defineProps({
    moduleIndex: { type: Number, default: 0 },
    modelValue: { type: Object, default: {} }
});

const productList = ref<any>([]);
const _getProductList = async () => {
    try {
        const result = await getHomeRecommendProductData({
            decorateId: homeStore.decorateId,
            previewId: homeStore.previewId,
            moduleIndex: props.moduleIndex
        });
        productList.value = result.catList;
    } catch (error) {}
};

onMounted(() => {
    _getProductList();
});
</script>
<style lang="less" scoped>
.simple-catbox {
    position: relative;
    margin: 24px 0;
    width: 100%;
    .cat-wrap {
        display: grid;
        grid-template-columns: repeat(6, minmax(0, 1fr));
        row-gap: 12px;
        .item {
            padding: 8px 4px;
            display: flex;
            flex-direction: column;
            min-height: 95px;
            box-sizing: border-box;
            align-items: center;
            &:hover {
                img {
                    transform: scale(1.2);
                }
                .tit {
                    font-weight: 700;
                }
            }
            .pic {
                width: 48px;
                height: 48px;
                img {
                    width: 100%;
                    height: 100%;
                    transition: all 0.4s ease 0s;
                }
            }
            .tit {
                margin-top: 10px;
                text-align: center;
                font-size: 14px;
                color: #333;
            }
        }
    }
}
</style>
