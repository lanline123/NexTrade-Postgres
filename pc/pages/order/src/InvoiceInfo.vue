<template>
    <div class="from">
        <div class="check-box">
            <Checkbox :checked="check"></Checkbox>
            <span>{{ $t("需要开发票") }}</span>
        </div>
        <div class="from_list" v-if="check">
            <div class="arrow"></div>
            <div class="invoice_from">
                <div class="from_item">
                    <div class="tit">{{ $t("发票类型") }}{{ $t("需要开发票") }}：</div>
                    <div class="content">
                        <div class="pay_type flex align-center">
                            <div
                                class="pay_tab flex align-center"
                                v-for="(item, index) in invoiceType"
                                :class="index == invoiceIndex ? 'active' : ''"
                                @click="checkInvoice(index)"
                            >
                                <span>{{ $t(item.name) }}</span>
                                <i class="icon-gou"></i>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="from_item">
                    <div class="tit">{{ $t("发票抬头") }}{{ $t("需要开发票") }}：</div>
                    <div class="content">
                        <input class="tig-input" type="text" :placeholder="$t('必填')" />
                    </div>
                </div>
                <div class="from_item" v-if="invoiceIndex == 0">
                    <div class="tit">{{ $t("企业税号") }}{{ $t("需要开发票") }}：</div>
                    <div class="content">
                        <input class="tig-ipnut" type="text" :placeholder="$t('必填')" />
                    </div>
                </div>
                <div class="from_item">
                    <div class="tit">{{ $t("发票内容") }}{{ $t("需要开发票") }}：</div>
                    <div class="content">
                        <div class="pay_type flex align-center">
                            <div
                                class="pay_tab flex align-center"
                                v-for="(item, index) in productType"
                                :class="index == productIndex ? 'active' : ''"
                                @click="checkProduct(index)"
                            >
                                <span>{{ $t(item.name) }}</span>
                                <i class="icon-gou"></i>
                            </div>
                        </div>
                    </div>
                </div>
                <div v-if="invoiceIndex == 1">
                    <!-- 无信息,新增 -->
                    <!-- <div>
                        <div class="from_item">
                            <div class="tit">增票资质信息<span class="red">（所有信息均为必填）</span></div>
                        </div>
                        <div class="from_item">
                            <div class="tit2">单位名称：</div>
                            <div class="content">
                                <RuleInput :styleInfo="styleInfo" msg="请填写完整单位名称"></RuleInput>
                            </div>
                        </div>
                        <div class="from_item">
                            <div class="tit2">纳税人识别码：</div>
                            <div class="content">
                                <RuleInput :styleInfo="styleInfo" msg="纳税人识别号不能为空" :ruleFn="aa"></RuleInput>
                                <p class="tips">请正确填写大小写字母</p>
                            </div>
                        </div>
                        <div class="from_item">
                            <div class="tit2">注册地址：</div>
                            <div class="content">
                                <RuleInput :styleInfo="styleInfo" msg="注册地址错误"></RuleInput>
                            </div>
                        </div>
                        <div class="from_item">
                            <div class="tit2">注册电话：</div>
                            <div class="content">
                                <RuleInput :styleInfo="styleInfo" msg="电话号码不能为空"></RuleInput>
                            </div>
                        </div>
                        <div class="from_item">
                            <div class="tit2">开户银行：</div>
                            <div class="content">
                                <RuleInput :styleInfo="styleInfo" msg="开户银行错误"></RuleInput>
                            </div>
                        </div>
                        <div class="from_item">
                            <div class="tit2">银行账户：</div>
                            <div class="content">
                                <RuleInput :styleInfo="styleInfo" msg="银行账号不能为空"></RuleInput>
                            </div>
                        </div>
                    </div> -->

                    <!-- 有信息,显示 -->
                    <div>
                        <div class="from_item">
                            <div class="tit">{{ $t("增票资质信息") }}{{ $t("需要开发票") }}</div>
                        </div>
                        <div class="from_item">
                            <div class="tit">{{ $t("单位名称") }}{{ $t("需要开发票") }}：</div>
                            <div class="content">
                                <div class="txt">131</div>
                            </div>
                        </div>
                        <div class="from_item">
                            <div class="tit">{{ $t("纳税人识别码") }}：</div>
                            <div class="content">
                                <div class="txt">D5S5W4DS5AD545WQ4DA</div>
                            </div>
                        </div>
                        <div class="from_item">
                            <div class="tit">{{ $t("注册地址") }}：</div>
                            <div class="content">
                                <div class="txt">3213</div>
                            </div>
                        </div>
                        <div class="from_item">
                            <div class="tit">{{ $t("注册电话") }}：</div>
                            <div class="content">
                                <div class="txt">23132****2</div>
                            </div>
                        </div>
                        <div class="from_item">
                            <div class="tit">{{ $t("开户银行") }}：</div>
                            <div class="content">
                                <div class="txt">32131</div>
                            </div>
                        </div>
                        <div class="from_item">
                            <div class="tit">{{ $t("银行账户") }}：</div>
                            <div class="content">
                                <div class="txt">213213****</div>
                            </div>
                        </div>
                    </div>

                    <!-- 地址信息, 已有 -->
                    <div style="margin-top: 30px">
                        <div class="from_item">
                            <div class="tit">{{ $t("增票收票地址") }} <span class="btn hand-pointer">{{ $t("修改收票地址") }}</span></div>
                        </div>
                        <div class="from_item">
                            <div class="tit">{{ $t("收票人姓名") }}：</div>
                            <div class="content">
                                <div class="txt">131</div>
                            </div>
                        </div>
                        <div class="from_item">
                            <div class="tit">{{ $t("收票人手机号") }}：</div>
                            <div class="content">
                                <div class="txt">158****0000</div>
                            </div>
                        </div>
                        <div class="from_item">
                            <div class="tit">{{ $t("收票人邮箱") }}：</div>
                            <div class="content">
                                <div class="txt">158****0000</div>
                            </div>
                        </div>
                        <div class="from_item">
                            <div class="tit">{{ $t("收票人省份") }}：</div>
                            <div class="content">
                                <div class="txt">甘肃 甘南 舟曲县</div>
                            </div>
                        </div>
                        <div class="from_item">
                            <div class="tit">{{ $t("收票人地址") }}：</div>
                            <div class="content">
                                <div class="txt">ffffsfvafdscsdfsaf</div>
                            </div>
                        </div>
                    </div>

                    <!-- 地址信息, 没有或者修改 -->
                    <!-- <div>
                        <div class="from_item2">
                            <div class="tit"><span class="red">* 注：我们会对您的专票资质进行复核，请确保信息准确。</span></div>
                            <div class="tit">审核结果查询地址：<span class="red">“会员中心->账户管理->增票资质”</span>。</div>
                        </div>
                        <div class="from_item2">
                            <div class="tit">增票收票地址</div>
                        </div>
                        <div class="from_item">
                            <div class="tit2">收票人姓名：</div>
                            <div class="content">
                                <RuleInput :styleInfo="styleInfo" msg="增票人姓名不能为空"></RuleInput>
                            </div>
                        </div>
                        <div class="from_item">
                            <div class="tit2">收票人手机号：</div>
                            <div class="content">
                                <RuleInput :styleInfo="styleInfo" msg="增票人姓名不能为空"></RuleInput>
                            </div>
                        </div>
                        <div class="from_item">
                            <div class="tit2">收票人邮箱：</div>
                            <div class="content">
                                <input class="tig-input" type="text">
                            </div>
                        </div>
                        <div class="from_item">
                            <div class="tit2">收票人省份：</div>
                            <div class="content">
                                <div>
                                    <select style="width:auto;" autocomplete="off" class="select tig-input" data-settings="{&quot;cutOff&quot;: 12}" data-type="2" data-nextid="selCities" name="province0" id="selProvinces0" onchange="regionChanged(this, 2, 'selCities0',false)">
                                        <option value="0">请选择省</option>
                                        <option value="2">北京</option>
                                        <option value="3">安徽</option>
                                        <option value="4">福建</option>
                                        <option value="5" selected="selected">甘肃</option>
                                        <option value="6">广东</option>
                                    </select>
                                    <select style="width:auto;" class="select tig-input" autocomplete="off" data-settings="{&quot;cutOff&quot;: 12}" name="city0" id="selCities0" onchange="regionChanged(this, 3, 'selDistricts0',false)">
                                        <option value="0">请选择市</option>
                                        <option value="62">兰州</option>
                                        <option value="64">定西</option>
                                        <option value="65" selected="selected">甘南</option>
                                        <option value="66">嘉峪关</option>
                                    </select>
                                    <select style="width:auto;" autocomplete="off" class="select tig-input" data-settings="{&quot;cutOff&quot;: 12}" name="district0" id="selDistricts0">
                                        <option value="0">请选择区</option>
                                        <option value="625">合作市</option>
                                        <option value="628" selected="selected">舟曲县</option>
                                        <option value="629">迭部县</option>
                                        <option value="630">玛曲县</option>
                                    </select>
                                </div>
                                <p class="tips">地址变更后您所有的增值税专用发票都将邮寄到新地址。</p>
                            </div>
                        </div>
                        <div class="from_item">
                            <div class="tit2">收票人地址：</div>
                            <div class="content">
                                <RuleInput :styleInfo="styleInfo" msg="收票地址不能为空"></RuleInput>
                            </div>
                        </div>
                    </div> -->
                </div>
            </div>
        </div>
    </div>
</template>
<script lang="ts" setup>
import { ref, onMounted, reactive } from "vue";
import { Checkbox } from "ant-design-vue";
import RuleInput from "./RuleInput.vue";
const styleInfo = reactive({
    height: "16px",
    width: "210px"
});
const aa = (value) => {
    if (value > 100) {
        return 1;
    } else {
        return "纳税人识别号错误，请检查";
    }
};
const check = ref(true);
const invoiceIndex = ref(0);
const invoiceType = reactive([
    {
        name: "普通发票"
    },
    {
        name: "增值税专用发票"
    }
]);
const checkInvoice = (index) => {
    invoiceIndex.value = index;
};

const productIndex = ref(0);
const productType = reactive([
    {
        name: "商品明细"
    },
    {
        name: "商品类别"
    }
]);
const checkProduct = (index) => {
    productIndex.value = index;
};
</script>
<style lang="less" scoped>
:deep(.ant-checkbox-checked .ant-checkbox-inner) {
    background-color: #ed3714 !important;
    border-color: #ed3714;
}

:deep(.ant-checkbox-checked:after) {
    border: none;
}
.check-box {
    line-height: 40px;
    span {
        margin-left: 10px;
        color: #333;
    }
}
.from_list {
    position: relative;
    border-radius: 3px;
    background-color: #fafafa;
    border: 1px solid #e6e6e6;
    padding: 20px 20px;
    margin-bottom: 10px;
    margin-top: 15px;
    .arrow {
        position: absolute;
        left: 20px;
        top: -10px;
        height: 20px;
        width: 20px;
        background: url("/assets/images/flow/ioc_arrow.gif") no-repeat 0 0;
    }
    .invoice_from {
        .from_item2 {
            .tit {
                color: #666;
                margin-bottom: 10px;
            }
        }
        .from_item {
            display: flex;
            // align-items: center;
            margin-bottom: 10px;
            .tit {
                color: #666;
                height: 16px;
                line-height: 16px;
                padding: 5px 0;
                .btn {
                    color: #09f;
                    margin-left: 10px;
                }
            }
            .tit2 {
                width: 100px;
                text-align: end;
                color: #666;
                height: 16px;
                line-height: 16px;
                padding: 5px 0;
            }
            .content {
                .pay_type {
                    .pay_tab {
                        cursor: pointer;
                        font-size: 12px;
                        padding: 5px 22px;
                        height: 16px;
                        line-height: 16px;
                        border: 1px solid #ddd;
                        background-color: #fff;
                        color: #666;
                        vertical-align: middle;
                        border-radius: 2px;
                        position: relative;
                        margin-right: 15px;

                        img {
                            display: none;
                            width: 13px;
                            height: 13px;
                            position: absolute;
                            right: 5px;
                        }

                        &:hover {
                            border-color: #ff5e5f;

                            img {
                                display: block;
                            }
                        }
                    }

                    .active {
                        border-color: #ff5e5f;

                        .icon-gou {
                            display: block;
                        }
                    }

                    .disabled {
                        border-color: #eee;
                        color: #b9b9b9;
                    }
                }
                input {
                    height: 16px;
                    line-height: 16px;
                }
                select {
                    margin-right: 5px;
                }
                .tips {
                    line-height: 30px;
                    color: #999;
                    font-weight: 400;
                }
                .txt {
                    height: 16px;
                    line-height: 16px;
                    padding: 6px 0;
                }
            }
        }
    }
}
</style>
