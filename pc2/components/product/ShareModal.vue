<template>
    <div class="share-box flex align-start">
        <div class="cd_r">
            <p>{{ $t("说说这个商品怎么样") }}？</p>

            <div class="text_box">
                <textarea id="share-contents" class="share-contents">{{ product.productName }}，{{ commonStore.dollarSign }}{{ productPrice }}</textarea>
            </div>
            <div class="show_box flex-wrap">
                <div class="show_item" v-for="(item, index) in picList" :class="picIndex == index ? 'redbd' : ''" @click="handlePic(index)">
                    <img :src="productImage(item)" style="width: 78px; margin-top: 0px" />
                    <i class="icon-gou"></i>
                </div>
            </div>
            <div class="share_box">
                <div class="s-icons-box">
                    <div class="s-icons flex">
                        <div class="lyecs-share flex" style="margin-bottom: 30px">
                            <a href="javascript:;" rel="nofollow" :title="$t('分享到微信')" class="weixin" @click="shareTo('wechat')">&nbsp;</a>
                            <a href="javascript:;" rel="nofollow" :title="$t('分享到新浪微博')" class="sina" @click="shareTo('weibo')">&nbsp;</a>
                            <a href="javascript:;" rel="nofollow" :title="$t('分享到QQ空间')" class="zone" @click="shareTo('qqzone')">&nbsp;</a>
                            <a href="javascript:;" rel="nofollow" :title="$t('分享到QQ')" class="qq" @click="shareTo('qq')">&nbsp;</a>
                        </div>
                        <div class="share_copy_link hand-pointer" @click="copy()">
                            {{ $t("复制链接") }}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
    <modal v-model:open="visible" :title="$t('扫描二维码分享')" :footer="null" style="top: 30%; width: 300px">
        <div>
            <img class="cd_ewm" :src="qrCodeData" width="100%" />
        </div>
    </modal>
</template>

<script lang="ts" setup>
import { reactive, ref, onMounted } from "vue";
import type { PicList, ProductItem } from "~/types/product/product.d";
import QRCode from "qrcode";
import { Modal } from "ant-design-vue";
import { productImage } from "@/utils/format";
import { useCommonStore } from "~/store/common";
const commonStore = useCommonStore();

const props = defineProps({
    picList: {
        type: Array as PropType<PicList[]>,
        default: []
    },
    product: {
        type: Object as PropType<ProductItem>,
        default: []
    },
    productPrice: {
        type: String,
        default: ""
    }
});
const picIndex = ref<any>(0);
const qrCodeData = ref<string>("");
const visible = ref<boolean>(false);
const shareTo = (type: string) => {
    if (type == "wechat") {
        visible.value = true;
    }
    if (type == "weibo") {
        // 构造微博分享链接
        let shareURL = "http://service.weibo.com/share/share.php?url=" + encodeURIComponent(props.product.productName + "-" + window.location.href);
        // 打开新窗口进行分享
        window.open(shareURL, "_blank");
    }
    if (type == "qqzone") {
        let title = props.product.productName;
        let url = window.location.href;
        let shareURL = `http://sns.qzone.qq.com/cgi-bin/qzshare/cgi_qzshare_onekey?to=qzone&title=${encodeURIComponent(title || "-")}&url=${encodeURIComponent(url)}`;
        window.open(shareURL, "_blank");
    }
    if (type == "qq") {
        let title = props.product.productName;
        let url = window.location.href;
        let shareURL = `http://connect.qq.com/widget/shareqq/index.html?url=${encodeURIComponent(url)}&title=${encodeURIComponent(title || "-")}`;
        window.open(shareURL, "_blank");
    }
};
const generateQRCodeImage = async (url: string) => {
    const data = url; // 替换为你要转换的链接
    // let opts = {
    //    errorCorrectionLevel: "L",//容错级别
    //    type: "image/png",//生成的二维码类型
    //    quality: 0.3,//二维码质量
    //    margin: 5,//二维码留白边距
    //    width: 128,//宽
    //    height: 128,//高
    //    text: "http://www.xxx.com",//二维码内容
    //    color: {
    //         dark: "#666666",//前景色
    //         light: "#fff"//背景色
    //    }
    // };
    let opts = {
        margin: 1 //二维码留白边距
    };
    qrCodeData.value = await QRCode.toDataURL(data, opts);
};
const handlePic = (index: number) => {
    picIndex.value = index;
};
const { t } = useI18n();
const copy = () => {
    window.prompt(t("请手动复制以下链接地址，按Ctrl+C即可复制"), window.location.href);
};
onMounted(() => {
    generateQRCodeImage(window.location.href);
});
</script>

<style lang="less" scoped>
.share-box {
    .cd_l {
        border-right: 1px solid #e6e6e6;
        padding-right: 20px;
        .img-box {
            position: relative;
            .cd_img {
                width: 310px;
                height: 310px;
            }
            .cd_ewm {
                position: absolute;
                bottom: 15px;
                left: 50%;
                margin-left: -51px;
            }
        }
    }
    .cd_r {
        margin-left: 20px;
        p {
            color: #6f6f6f;
            font-size: 14px;
            margin-bottom: 13px;
        }
        .text_box {
            .share-contents {
                border: 1px solid #e6e6e6;
                color: #b2b2b2;
                font-size: 12px;
                height: 66px;
                line-height: 22px;
                padding: 10px 10px 22px;
                resize: none;
                width: 92%;
            }
        }
        .show_box {
            margin-top: 15px;
            .show_item {
                position: relative;
                border: 1px solid #e6e6e6;
                cursor: pointer;
                display: block;
                height: 78px;
                margin-right: 5px;
                width: 78px;
                margin-bottom: 5px;
            }
            .redbd {
                border: 1px solid #cc0000 !important;
                .icon-gou {
                    display: block;
                }
            }
        }
        .share_box {
            margin-top: 25px;
            .share_copy_link {
                border: 1px solid #909090;
                color: #363636;
                font-size: 12px;
                line-height: 22px;
                height: 22px;
                text-align: center;
                width: 58px;
                margin-left: 30%;
            }
        }
    }
}
</style>
