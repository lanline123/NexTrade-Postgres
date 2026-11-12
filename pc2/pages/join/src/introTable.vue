<template>
    <div class="page-info">
        <div class="card container">
            <div class="step-section-title">1.{{ $t("选择主体类型和经营类目") }}</div>
            <div class="step-section-item">
                <div class="step-section-item__label">{{ $t("主体类型") }}</div>
                <div class="zent-form-control">
                    <div class="tag-list">
                        <div v-if="commonStore.personApplyEnabled == 1" :class="{ selected: type == 1 }" class="tag" @click="type = 1">{{ $t("个人") }}</div>
                        <div :class="{ selected: type == 2 }" class="tag" @click="type = 2">{{ $t("企业") }}</div>
                        <div :class="{ selected: type == 3 }" class="tag" @click="type = 3">{{ $t("个体工商户") }}</div>
                    </div>
                    <div class="cert-type-desc-principal">
                        <template v-if="type == 2">
                            {{ $t("“企业”在营业执照上的主体类型一般为：有限公司、有限责任公司等。") }}
                        </template>
                        <template v-else-if="type == 3">
                            {{ $t("“个体工商户”为持有个体工商户营业执照的商家，无需提供对公银行账户和开户许可证。") }}
                        </template>
                        <template v-else>
                            {{ $t("“个人”为根据法律法规和相关规定免于办理工商登记，无营业执照的商家。") }}
                        </template>
                    </div>
                </div>
            </div>
        </div>
        <div class="card container">
            <div class="step-section-title">2.{{ $t("按照以下清单准备材料") }}</div>
            <div class="step-section-item">
                <table class="zent-grid-table">
                    <colgroup>
                        <col style="width: 42px; min-width: 42px" />
                        <col style="width: 50%; min-width: 50%" />
                        <col style="width: 50%; min-width: 50%" />
                    </colgroup>
                    <thead class="zent-grid-thead">
                        <tr class="zent-grid-tr">
                            <th class="zent-grid-th zent-grid-text-align-center"></th>
                            <th class="zent-grid-th">
                                <div class="intro-column-title">{{ $t("认证材料") }}</div>
                            </th>
                            <th class="zent-grid-th">
                                <div class="intro-column-title">
                                    {{ $t("详细描述") }}<span class="sub-title">（{{ $t("复印件需加盖公司公章") }}）</span>
                                </div>
                            </th>
                        </tr>
                    </thead>
                    <tbody class="zent-grid-tbody">
                        <template v-if="type == 2 || type == 3">
                            <tr class="zent-grid-tr">
                                <td class="zent-grid-td zent-grid-text-align-center" rowspan="2">
                                    <span>{{ $t("主体资质") }}</span>
                                </td>
                                <td class="zent-grid-td">
                                    <a>{{ $t("统一社会信用代码证") }}</a>
                                </td>
                                <td class="zent-grid-td" data-zv="9.12.3">-</td>
                            </tr>
                            <tr class="zent-grid-tr">
                                <td class="zent-grid-td">
                                    <a>{{ $t("法人证件照正反面照片、法人手持证件照") }}</a
                                    ><br />
                                    <template v-if="type == 2"><a>{{ $t("开户许可证/基本存款账户照片") }}</a><br /></template>
                                </td>
                                <td class="zent-grid-td" data-zv="9.12.3">
                                    <p>· {{ $t("证件包含：身份证、护照、来往内地通行证、外国人居留证等有效证件。") }}</p>
                                    <p>· {{ $t("其中护照需要增加翻译件，加盖公司鲜章，外籍商家姓名需为中文（英文），如张三（Zhangsan）。") }}</p>
                                    <p>· {{ $t("照片标准：四角完整，清晰可辨。若加水印需保证照片重要信息清晰可辨。") }}</p>
                                    <p>· {{ $t("法人证件非大陆身份证（如护照等）时，需要提供法人证明函。") }}</p>
                                    <p v-if="type == 2">
                                        ·
                                        {{
                                            $t(
                                                "需提供与主体信息一致的开户许可证或基本存款账户照片，不支持借用其他主体的开户许可证或基本存款账户信息。如无法提供，也支持上传银行对公账户开户业务办理回执、网银截图、企业盖章的对公账户信息文件。"
                                            )
                                        }}
                                    </p>
                                    <p>{{ $t(" 注意：证件盖章不支持电子章，图片信息不得标有其他平台专用的水印/印章。") }}</p>
                                </td>
                            </tr>
                        </template>
                        <template v-else>
                            <tr class="zent-grid-tr">
                                <td class="zent-grid-td zent-grid-text-align-center" rowspan="1">
                                    <span>{{ $t("主体资质") }}</span>
                                </td>
                                <td class="zent-grid-td">
                                    <a>{{ $t("法定代表人证件照正反面照片") }}</a
                                    ><span></span>
                                </td>
                                <td class="zent-grid-td">
                                    <p>· {{ $t("证件包含：身份证、护照、来往内地通行证、外国人居留证等有效证件。") }}</p>
                                    <p>· {{ $t("其中护照需要增加翻译件，加盖公司鲜章，外籍商家姓名需为中文（英文），如张三（Zhangsan）。") }}</p>
                                    <p>· {{ $t("照片标准：四角完整，清晰可辨。若加水印需保证照片重要信息清晰可辨。") }}</p>
                                    <p>· {{ $t("法人证件非大陆身份证（如护照等）时，需要提供法人证明函。") }}</p>
                                    <p>· {{ $t("注意：证件盖章不支持电子章，图片信息不得标有其他平台专用的水印/印章。") }}</p>
                                </td>
                            </tr>
                        </template>
                    </tbody>
                </table>
            </div>
        </div>
        <div class="card container bw">
            <slot></slot>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { useCommonStore } from "~/store/common";

const type = defineModel<number>("type");
const commonStore = useCommonStore();
</script>
<style lang="less" scoped>
.page-info {
    display: flex;
    gap: 26px;
    align-items: center;
    flex-direction: column;
    padding: 26px 0;
    box-sizing: border-box;

    .card {
        background-color: #ffffff;
        display: flex;
        padding: 24px;
        flex-direction: column;

        .step-section-title {
            margin-bottom: 24px;
            font-size: 20px;
            line-height: 28px;
            color: #414142;
        }

        .step-section-item {
            display: flex;
            margin-bottom: 24px;

            .step-section-item__label {
                margin-right: 16px;
                min-width: 64px;
                font-size: 16px;
                line-height: 40px;
                color: #646566;
                text-align: right;
            }

            .zent-form-control {
                display: flex;
                flex-direction: column;
                gap: 16px;

                .tag-list {
                    display: flex;
                    flex-wrap: wrap;
                    gap: 8px;

                    .tag {
                        border-radius: 2px;
                        padding: 0 10px;
                        font-size: 16px;
                        color: #323233;
                        min-width: 100px;
                        height: 40px;
                        border: 1px solid #ebedf0;
                        margin-right: 16px;
                        position: relative;
                        cursor: pointer;
                        box-sizing: border-box;
                        text-align: center;
                        line-height: 40px;
                    }

                    .selected {
                        border-color: var(--general); /* 选中状态下边框变红 */
                    }

                    .selected::after {
                        content: "";
                        position: absolute;
                        bottom: 0; /* 三角形在右下角，但留有一点间距 */
                        right: 0;
                        width: 0;
                        height: 0;
                        border-left: 20px solid transparent;
                        border-top: 20px solid transparent;
                        border-bottom: 20px solid var(--general); /* 三角形大小和颜色 */
                        font-size: 20px; /* 勾的大小 */
                        line-height: 40px; /* 根据三角形大小调整，确保勾垂直居中 */
                        text-align: center;
                        transform: rotate(-0deg); /* 调整勾的位置和角度 */
                    }

                    .selected::before {
                        content: "✔";
                        color: var(--main-text); /* 勾的颜色 */
                        font-size: 10px; /* 勾的大小 */
                        position: absolute;
                        bottom: 5px; /* 调整勾在三角形内的垂直位置 */
                        right: 0; /* 调整勾在三角形内的水平位置 */
                        transform: translateX(0%) translateY(50%); /* 微调勾的位置 */
                        z-index: 100;
                    }
                }

                .cert-type-desc-principal {
                    padding: 16px;
                    font-size: 14px;
                    background-color: #f7f8fa;
                    color: #646566;
                    line-height: 20px;
                    box-sizing: border-box;
                }
            }

            .zent-grid-table {
                border-collapse: collapse;

                colgroup {
                    display: table-column-group;
                    unicode-bidi: isolate;

                    col {
                        display: table-column;
                        unicode-bidi: isolate;
                    }
                }

                .zent-grid-thead {
                    .zent-grid-tr {
                        background-color: #f7f7f7;
                        display: table-row;
                        height: 56px;
                    }
                }

                tr {
                    border: 1px solid #ebedf0;

                    th {
                        border: 1px solid #ebedf0;
                        padding: 18px 16px;
                    }

                    td {
                        border: 1px solid #ebedf0;
                        padding: 18px 16px;
                        font-size: 14px;
                        line-height: 1.8;
                    }
                }

                .intro-column-title {
                    font-size: 16px;
                    line-height: 22px;
                    color: #39393a;
                    font-weight: bold;

                    .sub-title {
                        font-size: 14px;
                        line-height: 20px;
                        color: #9b9c9d;
                        font-weight: normal;
                    }
                }

                .zent-grid-text-align-center {
                }
            }
        }
    }

    .bw {
        justify-content: space-between;
        flex-direction: row;
        align-items: center;

        .xieyi {
            display: flex;
            justify-content: center;
            align-items: center;

            .url {
                font-size: 14px;
                cursor: pointer;
            }
        }
    }
}
</style>
