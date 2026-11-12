<template>
    <div class="navigation-row flex align-center">
        <div class="nav-tit flex align-center" v-if="navInfo.catname">
            <a>{{ navInfo.catname }}</a>
            <i class="crumbs-arrow">></i>
        </div>
        <div class="nav-select-row" v-for="nav in navInfo.navList">
            <div class="nav-select flex align-center">
                <div class="select-item flex align-center">
                    <p>{{ nav.name }}</p>
                    <i class="iconfont-pc icon-xiajiantou"></i>
                </div>
                <i class="crumbs-arrow">></i>
            </div>
            <div class="pop-up-conter">
                <div class="pop-up flex justify-between">
                    <a v-for="item in nav.arr" href="#">{{ item }}</a>
                </div>
            </div>
        </div>
        <div class="nav-txt" v-if="navInfo.productName">
            <p class="ellipsis">{{ navInfo.productName }}</p>
        </div>
        <div class="nav-selected" v-if="navInfo.selected.length > 0 && 0">
            <a class="item" rel="nofollow" href="#" v-for="sel in navInfo.selected"
                ><span>{{ sel.name }}：</span>{{ sel.arr.join(",") }}</a
            >
            <a class="item" rel="nofollow" href="https://demo2.lyecs.com/list.html?cat=416">{{ $t("清除全部") }}</a>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref, shallowRef, onMounted, reactive } from "vue";
const navInfo = reactive({
    catname: "家电",
    cat: 9,
    navList: [
        {
            name: "3C数码",
            arr: ["3C数码", "空调", "冰箱", "厨卫大电"]
        },
        {
            name: "手机",
            arr: ["手机", "平板电脑", "笔记本", "智能手表", "智能手环", "电子辞典"]
        }
    ],
    selected: [
        {
            name: "价格",
            arr: ["120-300"]
        },
        {
            name: "品牌",
            arr: ["苹果", "华为"]
        },
        {
            name: "CPU型号",
            arr: ["晓龙8系列", "骁龙782"]
        }
    ],
    productName: "Apple iPhone 14 Pro Max (A2896) 256GB 暗紫色 支持移动联通电信5G 双卡双待手机"
});
</script>
<style lang="less" scoped>
.navigation-row {
    padding-top: 15px;
    cursor: pointer;
    i {
        font-family: simsun;
        font-style: normal;
        color: #333;
        font-size: 12px;
        margin: 0 5px;
    }
    .nav-tit {
        a {
            font-family: "微软雅黑";
            font-size: 18px;
            color: #666666;
            font-weight: 700;
            outline: none;
        }
        a:hover {
            color: #666 !important;
        }
    }
    .nav-select-row {
        position: relative;
        .nav-select {
            i {
                font-family: simsun;
                font-style: normal;
                color: #666;
                font-size: 12px;
            }
            .select-item {
                border: 1px solid #dddddd;
                height: 22px;
                line-height: 22px;
                padding: 0 4px 0 8px;
            }
        }
        .pop-up-conter {
            background-color: #ffffff;
            border: 1px solid #dddddd;
            display: none;
            left: 0;
            padding: 10px;
            position: absolute;
            top: 23px;
            width: 350px;
            .pop-up {
                flex-wrap: wrap;

                a {
                    height: 24px;
                    line-height: 24px;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    white-space: nowrap;
                    width: 65px;
                }
            }
        }
        &:hover {
            .nav-select {
                .select-item {
                    border-color: #e4393c;
                    border-bottom-color: #ffffff;
                    position: relative;
                    z-index: 99;

                    .icon-xiajiantou {
                        transform: rotate(180deg);
                    }
                }
            }
            .pop-up-conter {
                display: block;
                border-color: #e4393c;
                z-index: 88;
            }
        }
    }
    .nav-txt {
        color: #888;
        width: 260px;
    }
    .nav-selected {
        a {
            display: inline-block;
            white-space: nowrap;
            border: 1px solid #dddddd;
            color: #ff4040;
            padding: 0 24px 0 8px;
            margin-right: 8px;
            background: url("/assets/images/common/icons.png") no-repeat scroll right -247px #fff;
            height: 22px;
            line-height: 22px;
            &:hover {
                background-color: #f2f2f2;
                background-position: right -219px;
                text-decoration: none;
            }
        }
        span {
            color: #666;
        }
    }
}
</style>
