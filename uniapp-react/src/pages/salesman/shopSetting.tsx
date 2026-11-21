import { useEffect, useState } from "react";
import { Button, Input, Text, Textarea, View } from "@tarojs/components";
import Taro from "@tarojs/taro";

import TigImage from "@/components/tig/tig-image/TigImage";
import TigLayout from "@/components/tig/tig-layout/TigLayout";
import TigMediaUpload, { type MediaUploadItem } from "@/components/tig/tig-media-upload/TigMediaUpload";
import { getQrCode } from "@/api/common";
import { getMyShellShop, saveMyShellShop, type ShellShopMine } from "@/api/salesman/salesmanShop";
import { useConfigStore } from "@/store/config";
import { t } from "@/utils/i18n";
import { usePageGuards } from "@/utils/pageMixin";

import "./shopSetting.scss";

/**
 * 分销员外壳店「店中店」自助设置（我方新增，论坛帖 257876041323848173）。
 * 可自定义仅三项：店名 / 店招图 / 简介（主题色等扩展项本批不做）。
 * 保存只写外壳呈现配置；专属链接 / 二维码与分销员名片同口径
 * （h5Domain + salesmanId 公开标识），买家进入后的归因走既有链。
 */
export default function ShellShopSetting() {
    usePageGuards();

    const h5Domain = useConfigStore((s) => s.baseInfo.h5Domain);

    const [mine, setMine] = useState<ShellShopMine | null>(null);
    const [notSalesman, setNotSalesman] = useState(false);
    const [shopName, setShopName] = useState("");
    const [intro, setIntro] = useState("");
    const [bannerList, setBannerList] = useState<MediaUploadItem[]>([]);
    const [saving, setSaving] = useState(false);
    const [qrImg, setQrImg] = useState("");

    const link = (() => {
        if (!mine) return "";
        let domain = h5Domain;
        // #ifdef H5 同名片页口径
        if (process.env.TARO_ENV === "h5") {
            domain = domain ? domain : `${window.location.origin}/mobile`;
        }
        return `${domain}/pages/salesman/shop?salesmanId=${mine.salesmanId}`;
    })();

    useEffect(() => {
        void (async () => {
            try {
                const result = await getMyShellShop();
                if (result) {
                    setMine(result);
                    setShopName(result.shopName || "");
                    setIntro(result.intro || "");
                    setBannerList(result.banner ? [{ url: result.banner, picUrl: result.banner }] : []);
                }
            } catch (error: any) {
                console.error(error);
                // 非分销员：后端业务错误「当前用户不是分销员」
                if (String(error?.message || "").includes("不是分销员")) {
                    setNotSalesman(true);
                } else {
                    Taro.showToast({ title: t("加载失败"), icon: "none" });
                }
            }
        })();
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    useEffect(() => {
        if (!link) return;
        void (async () => {
            try {
                const res: any = await getQrCode(link);
                setQrImg(typeof res === "string" ? res : res?.qrCode || res?.url || "");
            } catch (error) {
                console.error(error);
            }
        })();
    }, [link]);

    const handleSave = async () => {
        if (saving) return;
        setSaving(true);
        try {
            const bannerItem = bannerList[0];
            const result = await saveMyShellShop({
                shopName: shopName.trim(),
                banner: bannerItem?.picUrl || bannerItem?.url || "",
                intro: intro.trim()
            });
            if (result) setMine(result);
            Taro.showToast({ title: t("保存成功"), icon: "success" });
        } catch (error: any) {
            Taro.showToast({ title: error?.message || t("保存失败"), icon: "none" });
        } finally {
            setSaving(false);
        }
    };

    const copyLink = () => {
        if (!link) return;
        Taro.setClipboardData({
            data: link,
            success: () => Taro.showToast({ title: t("链接已复制"), icon: "success" })
        });
    };

    const goPreview = () => {
        if (!mine) return;
        Taro.navigateTo({ url: `/pages/salesman/shop?salesmanId=${mine.salesmanId}` });
    };

    return (
        <View>
            <TigLayout title={t("我的小店")} safeTopBgColor="#f5f5f5">
                {notSalesman ? (
                    <View className="shell-setting-state">
                        <Text>{t("成为分销员后即可开通自己的小店")}</Text>
                    </View>
                ) : !mine ? (
                    <View className="shell-setting-state">
                        <Text>{t("加载中…")}</Text>
                    </View>
                ) : (
                    <>
                        {!mine.enabled ? (
                            <View className="shell-setting-notice">
                                <Text>{t("平台已暂停你的小店展示，设置仍可保存，恢复展示后生效。")}</Text>
                            </View>
                        ) : null}

                        <View className="shell-setting-card">
                            <View className="shell-setting-row">
                                <View className="shell-setting-label">{t("店名")}</View>
                                <Input
                                    className="shell-setting-input"
                                    value={shopName}
                                    maxlength={50}
                                    placeholder={t("不填则展示你的昵称")}
                                    onInput={(e) => setShopName(e.detail.value)}
                                />
                            </View>
                            <View className="shell-setting-row shell-setting-row--block">
                                <View className="shell-setting-label">{t("店招图")}</View>
                                <TigMediaUpload limit={1} modelValue={bannerList} onModelValueChange={setBannerList} trigger="plus" />
                            </View>
                            <View className="shell-setting-row shell-setting-row--block">
                                <View className="shell-setting-label">{t("简介")}</View>
                                <Textarea
                                    className="shell-setting-textarea"
                                    value={intro}
                                    maxlength={200}
                                    placeholder={t("介绍一下你的小店（200字内）")}
                                    onInput={(e) => setIntro(e.detail.value)}
                                />
                            </View>
                            <Button className="shell-setting-save" loading={saving} onClick={handleSave}>
                                {t("保存")}
                            </Button>
                        </View>

                        <View className="shell-setting-card">
                            <View className="shell-setting-label">{t("我的小店链接")}</View>
                            <View className="shell-setting-link">{link}</View>
                            <View className="shell-setting-actions">
                                <Button className="shell-setting-action" onClick={copyLink}>
                                    {t("复制链接")}
                                </Button>
                                <Button className="shell-setting-action" onClick={goPreview}>
                                    {t("预览小店")}
                                </Button>
                            </View>
                            {qrImg ? (
                                <View className="shell-setting-qr">
                                    <TigImage src={qrImg} />
                                    <Text className="shell-setting-qr-tip">{t("扫码进入我的小店")}</Text>
                                </View>
                            ) : null}
                        </View>
                    </>
                )}
            </TigLayout>
        </View>
    );
}
