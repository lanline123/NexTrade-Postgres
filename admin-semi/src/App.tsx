import { lazy, Suspense, useEffect, useState, type ReactNode } from 'react';
import { apiGet, clearToken, enterDemo, getToken, isDemo } from './api';
import { applyAppearance, saveAppearance, type AppearanceSettings } from './theme';
import AdminLayout, { type PageKey } from './layout/AdminLayout';
import {
  resolveMallUrl,
  setMallSettings as cacheMallSettings,
  type MallSettings,
} from './mall';
import { CountrySelectField, ShopRegionCascaderField } from './components/RegionSettingFields';
const Login = lazy(() => import('./pages/Login'));
const Dashboard = lazy(() => import('./pages/Dashboard'));
const Products = lazy(() => import('./pages/Products'));
const Orders = lazy(() => import('./pages/Orders'));
const Members = lazy(() => import('./pages/Members'));
const Content = lazy(() => import('./pages/Content'));
const Brands = lazy(() => import('./pages/Brands'));
const ProductGroups = lazy(() => import('./pages/ProductGroups'));
const Categories = lazy(() => import('./pages/Categories'));
const ProductServices = lazy(() => import('./pages/ProductServices'));
const Comments = lazy(() => import('./pages/Comments'));
const AttributesTemplates = lazy(() => import('./pages/AttributesTemplates'));
const FriendLinks = lazy(() => import('./pages/FriendLinks'));
const Regions = lazy(() => import('./pages/Regions'));
const LogisticsCompanies = lazy(() => import('./pages/LogisticsCompanies'));
const AppVersion = lazy(() => import('./pages/AppVersion'));
const SystemUpdate = lazy(() => import('./pages/SystemUpdate'));
const ClientRelease = lazy(() => import('./pages/ClientRelease'));
const AdminLogs = lazy(() => import('./pages/AdminLogs'));
const AdminUsers = lazy(() => import('./pages/AdminUsers'));
const Roles = lazy(() => import('./pages/Roles'));
const AgreementPage = lazy(() => import('./pages/AgreementPage'));
const AccountManage = lazy(() => import('./pages/AccountManage'));
const TaxRates = lazy(() => import('./pages/TaxRates'));
const ShippingTypes = lazy(() => import('./pages/ShippingTypes'));
const ShippingSettingsPage = lazy(() => import('./pages/ShippingSettings'));
import type { ConfigRendererContext, ConfigFieldGroup, ConfigFieldRule, ConfigNumberLimit } from './pages/AutoConfigPage';
const AutoConfigPage = lazy(() => import('./pages/AutoConfigPage'));
const BusinessLicenseConfigField = lazy(() => import('./pages/AutoConfigPage').then((m) => ({ default: m.BusinessLicenseConfigField })));
const CopyrightConfigField = lazy(() => import('./pages/AutoConfigPage').then((m) => ({ default: m.CopyrightConfigField })));
const TechSupportConfigField = lazy(() => import('./pages/AutoConfigPage').then((m) => ({ default: m.TechSupportConfigField })));
import MultilingualInput from './components/MultilingualInput';
import AfterSalesReasonField from './config/AfterSalesReasonField';
const LogisticsSettings = lazy(() => import('./pages/LogisticsSettings'));
const PayLogs = lazy(() => import('./pages/PayLogs'));
const RefundLogs = lazy(() => import('./pages/RefundLogs'));
const BalanceLogs = lazy(() => import('./pages/BalanceLogs'));
const StaffWalletLogs = lazy(() => import('./pages/StaffWalletLogs'));
const BuyerShow = lazy(() => import('./pages/BuyerShow'));
const IntegralLogs = lazy(() => import('./pages/IntegralLogs'));
const RefundApplyList = lazy(() => import('./pages/FinanceLists').then((m) => ({ default: m.RefundApplyList })));
const WithdrawApplyList = lazy(() => import('./pages/FinanceLists').then((m) => ({ default: m.WithdrawApplyList })));
const RechargeOrderList = lazy(() => import('./pages/FinanceLists').then((m) => ({ default: m.RechargeOrderList })));
const UserInvoiceList = lazy(() => import('./pages/FinanceLists').then((m) => ({ default: m.UserInvoiceList })));
const OrderInvoiceList = lazy(() => import('./pages/FinanceLists').then((m) => ({ default: m.OrderInvoiceList })));
const DepositRules = lazy(() => import('./pages/FinanceMoreLists').then((m) => ({ default: m.DepositRules })));
const DepositAccounts = lazy(() => import('./pages/FinanceMoreLists').then((m) => ({ default: m.DepositAccounts })));
const DepositQuery = lazy(() => import('./pages/FinanceMoreLists').then((m) => ({ default: m.DepositQuery })));
const DepositAudit = lazy(() => import('./pages/FinanceMoreLists').then((m) => ({ default: m.DepositAudit })));
const DepositLogs = lazy(() => import('./pages/FinanceMoreLists').then((m) => ({ default: m.DepositLogs })));
const CategoryFeeRules = lazy(() => import('./pages/FinanceMoreLists').then((m) => ({ default: m.CategoryFeeRules })));
const MerchantApplications = lazy(() => import('./pages/FinanceMoreLists').then((m) => ({ default: m.MerchantApplications })));
const SubMerchants = lazy(() => import('./pages/FinanceMoreLists').then((m) => ({ default: m.SubMerchants })));
const ProfitSharing = lazy(() => import('./pages/FinanceMoreLists').then((m) => ({ default: m.ProfitSharing })));
const StatementDownload = lazy(() => import('./pages/FinanceMoreLists').then((m) => ({ default: m.StatementDownload })));
const PosWorkbench = lazy(() => import('./pages/PosPages').then((m) => ({ default: m.PosWorkbench })));
const CashierDevices = lazy(() => import('./pages/PosPages').then((m) => ({ default: m.CashierDevices })));
const PosCashierList = lazy(() => import('./pages/PosCashierList'));
const ReceiptPrintersPage = lazy(() => import('./pages/ReceiptPrinters'));
const PaymentSettings = lazy(() => import('./pages/PaymentSettings'));
const NoticeSettings = lazy(() => import('./pages/NoticeSettings'));
const WithdrawalSettings = lazy(() => import('./pages/WithdrawalSettings'));
const ProfitSharingSettings = lazy(() => import('./pages/ProfitSharingSettings'));
const DepositSettings = lazy(() => import('./pages/DepositSettings'));
const PriceDisplaySettings = lazy(() => import('./pages/PriceDisplaySettings'));
const PaidCancelRefundSettings = lazy(() => import('./pages/PaidCancelRefundSettings'));
const GiftOrderSettings = lazy(() => import('./pages/GiftOrderSettings'));
const KitchenBoard = lazy(() => import('./pages/KitchenBoard'));
const AdCampaignManage = lazy(() => import('./pages/AdCampaignManage'));
const AdSettingsPage = lazy(() => import('./pages/AdSettings'));
const AdPromotionManage = lazy(() => import('./pages/selfService/AdPromotion'));
const BuyStyleSettings = lazy(() => import('./pages/BuyStyleSettings'));
const SystemModeSettings = lazy(() => import('./pages/SystemModeSettings'));
const FederationSettings = lazy(() => import('./pages/FederationSettings'));
const SiteClone = lazy(() => import('./pages/SiteClone'));
const TakeoutManage = lazy(() => import('./pages/TakeoutManage'));
const StoreMapDistribution = lazy(() => import('./pages/StoreMapDistribution'));
const PlatformAnnouncements = lazy(() => import('./pages/PlatformAnnouncements'));
const OrgOverview = lazy(() => import('./pages/OrgOverview'));
const HubLinks = lazy(() => import('./pages/HubLinks'));
const HubPairing = lazy(() => import('./pages/HubPairing'));
const MyLinks = lazy(() => import('./pages/MyLinks'));
const HubCredentials = lazy(() => import('./pages/HubCredentials'));
const HubAggregation = lazy(() => import('./pages/HubAggregation'));
const HubMembers = lazy(() => import('./pages/HubMembers'));
const HubOrders = lazy(() => import('./pages/HubOrders'));
const HubSso = lazy(() => import('./pages/HubSso'));
const HubConfig = lazy(() => import('./pages/HubConfig'));
const FeedKeys = lazy(() => import('./pages/FeedKeys'));
const FontSettings = lazy(() => import('./pages/FontSettings'));
import { applyAdminFont, type FontSettingsConfig } from './fontRegistry';
const KefuSettings = lazy(() => import('./pages/KefuSettings'));
const ShopSettings = lazy(() => import('./pages/ShopSettings'));
const StoreSettings = lazy(() => import('./pages/StoreSettings'));
const StoreTips = lazy(() => import('./pages/StoreTips'));
const StoreAreas = lazy(() => import('./pages/StoreAreas'));
const StoreList = lazy(() => import('./pages/StoreList'));
const SelfPackList = lazy(() => import('./pages/SelfPackList'));
const DeliverymanList = lazy(() => import('./pages/StaffPages').then((m) => ({ default: m.DeliverymanList })));
const ServicemanList = lazy(() => import('./pages/StaffPages').then((m) => ({ default: m.ServicemanList })));
const StaffApplyManage = lazy(() => import('./pages/RiderManagePages').then((m) => ({ default: m.StaffApplyManage })));
const DeliveryOrderManage = lazy(() => import('./pages/RiderManagePages').then((m) => ({ default: m.DeliveryOrderManage })));
const RiderTrackManage = lazy(() => import('./pages/RiderManagePages').then((m) => ({ default: m.RiderTrackManage })));
const DeliveryStaffWithdraw = lazy(() => import('./pages/StaffWithdrawPages').then((m) => ({ default: m.DeliveryStaffWithdraw })));
const ServerStaffWithdraw = lazy(() => import('./pages/StaffWithdrawPages').then((m) => ({ default: m.ServerStaffWithdraw })));
const Wholesalers = lazy(() => import('./pages/Wholesalers').then((m) => ({ default: m.Wholesalers })));
const WholesalerGrades = lazy(() => import('./pages/WholesalerGrades').then((m) => ({ default: m.WholesalerGrades })));
const WholesaleSetting = lazy(() => import('./pages/WholesaleSetting').then((m) => ({ default: m.WholesaleSetting })));
const WholesaleProducts = lazy(() => import('./pages/WholesaleProducts').then((m) => ({ default: m.WholesaleProducts })));
const WholesaleSummary = lazy(() => import('./pages/WholesaleSummary').then((m) => ({ default: m.WholesaleSummary })));
const StorePickupSetting = lazy(() => import('./pages/StorePickupSetting').then((m) => ({ default: m.StorePickupSetting })));
const StoreAppointmentTemplates = lazy(() => import('./pages/StoreAppointmentTemplates').then((m) => ({ default: m.StoreAppointmentTemplates })));
const StoreEntryRuleSetting = lazy(() => import('./pages/StoreEntryRuleSetting').then((m) => ({ default: m.StoreEntryRuleSetting })));
const WechatLiveMiniAppSetting = lazy(() => import('./pages/WechatLiveMiniAppSetting').then((m) => ({ default: m.WechatLiveMiniAppSetting })));
const ShippingTemplates = lazy(() => import('./pages/ShippingTemplates').then((m) => ({ default: m.ShippingTemplates })));
const MessageTypes = lazy(() => import('./pages/MessageTypes').then((m) => ({ default: m.MessageTypes })));
const SalesmanOverview = lazy(() => import('./pages/SalesmanOverview'));
const SalesmanAnalysisTable = lazy(() => import('./pages/SalesmanOverview').then((m) => ({ default: m.SalesmanAnalysisTable })));
const SalesmanRankingTable = lazy(() => import('./pages/SalesmanOverview').then((m) => ({ default: m.SalesmanRankingTable })));
const SalesmanSubgroup = lazy(() => import('./pages/SalesmanSubgroup'));
const SalesmanNotice = lazy(() => import('./pages/SalesmanContent').then((m) => ({ default: m.SalesmanNotice })));
const SalesmanMaterialCategory = lazy(() => import('./pages/SalesmanContent').then((m) => ({ default: m.SalesmanMaterialCategory })));
const SalesmanMaterial = lazy(() => import('./pages/SalesmanContent').then((m) => ({ default: m.SalesmanMaterial })));
const SalesmanDetailed = lazy(() => import('./pages/SalesmanDetailedPages').then((m) => ({ default: m.SalesmanDetailed })));
const CustomerTransactions = lazy(() => import('./pages/SalesmanDetailedPages').then((m) => ({ default: m.CustomerTransactions })));
const SalesmanSettlementSetting = lazy(() => import('./pages/SalesmanSettlementPages').then((m) => ({ default: m.SalesmanSettlementSetting })));
const SalesmanPerformanceSettlement = lazy(() => import('./pages/SalesmanSettlementPages').then((m) => ({ default: m.SalesmanPerformanceSettlement })));
const SalesmanPromoteMode = lazy(() => import('./pages/SalesmanPromoteAndProducts').then((m) => ({ default: m.SalesmanPromoteMode })));
const SalesmanProducts = lazy(() => import('./pages/SalesmanPromoteAndProducts').then((m) => ({ default: m.SalesmanProducts })));
const SalesmanPromoters = lazy(() => import('./pages/SalesmanPromoteAndProducts').then((m) => ({ default: m.SalesmanPromoters })));
const LanguagesList = lazy(() => import('./pages/MultiLanguagePages').then((m) => ({ default: m.LanguagesList })));
const CurrencyManagement = lazy(() => import('./pages/MultiLanguagePages').then((m) => ({ default: m.CurrencyManagement })));
const TranslationContent = lazy(() => import('./pages/MultiLanguagePages').then((m) => ({ default: m.TranslationContent })));
const VerbalAssociation = lazy(() => import('./pages/MultiLanguagePages').then((m) => ({ default: m.VerbalAssociation })));
const AreaCode = lazy(() => import('./pages/MultiLanguagePages').then((m) => ({ default: m.AreaCode })));
const AiImageSetting = lazy(() => import('./pages/MultiLanguagePages').then((m) => ({ default: m.AiImageSetting })));
const IntraCityDelivery = lazy(() => import('./pages/O2ODeliveryServicePages').then((m) => ({ default: m.IntraCityDelivery })));
const HomeService = lazy(() => import('./pages/O2ODeliveryServicePages').then((m) => ({ default: m.HomeService })));
const PremiumMembers = lazy(() => import('./pages/PremiumMemberPages').then((m) => ({ default: m.PremiumMembers })));
const PremiumMemberRights = lazy(() => import('./pages/PremiumMemberPages').then((m) => ({ default: m.PremiumMemberRights })));
const SupplierList = lazy(() => import('./pages/SupplyChainSupplierPages').then((m) => ({ default: m.SupplierList })));
const SupplierCategories = lazy(() => import('./pages/SupplyChainSupplierPages').then((m) => ({ default: m.SupplierCategories })));
const SupplierProducts = lazy(() => import('./pages/SupplyChainSupplierPages').then((m) => ({ default: m.SupplierProducts })));
const PurchaseSetting = lazy(() => import('./pages/SupplyChainSupplierPages').then((m) => ({ default: m.PurchaseSetting })));
const PurchaseOrders = lazy(() => import('./pages/SupplyChainPurchasePages').then((m) => ({ default: m.PurchaseOrders })));
const PurchaseInbound = lazy(() => import('./pages/SupplyChainPurchasePages').then((m) => ({ default: m.PurchaseInbound })));
const PurchaseReturns = lazy(() => import('./pages/SupplyChainPurchasePages').then((m) => ({ default: m.PurchaseReturns })));
const PurchaseReconciliation = lazy(() => import('./pages/SupplyChainPurchasePages').then((m) => ({ default: m.PurchaseReconciliation })));
const ShopTipManage = lazy(() => import('./pages/OverseasExtraPages').then((m) => ({ default: m.ShopTipManage })));
const ShopGroupManage = lazy(() => import('./pages/OverseasExtraPages').then((m) => ({ default: m.ShopGroupManage })));
const OfflinePaymentClaimManage = lazy(() => import('./pages/OverseasExtraPages').then((m) => ({ default: m.OfflinePaymentClaimManage })));
const ImageSearchManage = lazy(() => import('./pages/OverseasExtraPages').then((m) => ({ default: m.ImageSearchManage })));
const VendorAccountManage = lazy(() => import('./pages/OverseasExtraPages').then((m) => ({ default: m.VendorAccountManage })));
const VendorFundsManage = lazy(() => import('./pages/OverseasExtraPages').then((m) => ({ default: m.VendorFundsManage })));
const VendorFinancialLogManage = lazy(() => import('./pages/OverseasExtraPages').then((m) => ({ default: m.VendorFinancialLogManage })));
const ShopAccountOverview = lazy(() => import('./pages/ShopFundsPages').then((m) => ({ default: m.ShopAccountOverview })));
const ShopFundsManage = lazy(() => import('./pages/ShopFundsPages').then((m) => ({ default: m.ShopFundsManage })));
const ShopFinancialLogManage = lazy(() => import('./pages/ShopFundsPages').then((m) => ({ default: m.ShopFinancialLogManage })));
const LicensedPage = lazy(() => import('./pages/LicensedPage'));
const KefuWorkbench = lazy(() => import('./pages/KefuWorkbench').then((m) => ({ default: m.KefuWorkbench })));
const KefuWorkbenchStandalone = lazy(() => import('./pages/KefuWorkbench').then((m) => ({ default: m.KefuWorkbenchStandalone })));
const KefuH5Workbench = lazy(() => import('./pages/KefuH5').then((m) => ({ default: m.KefuH5Workbench })));
const KefuHistory = lazy(() => import('./pages/KefuWorkbench').then((m) => ({ default: m.KefuHistory })));
const ShopInfoDashboard = lazy(() => import('./pages/MerchantVendorExtras').then((m) => ({ default: m.ShopInfoDashboard })));
const ShopStaffManage = lazy(() => import('./pages/MerchantVendorExtras').then((m) => ({ default: m.ShopStaffManage })));
const ShopTeamSettings = lazy(() => import('./pages/MerchantVendorExtras').then((m) => ({ default: m.ShopTeamSettings })));
const VendorBindShopManage = lazy(() => import('./pages/MerchantVendorExtras').then((m) => ({ default: m.VendorBindShopManage })));
const VendorStaffManage = lazy(() => import('./pages/MerchantVendorExtras').then((m) => ({ default: m.VendorStaffManage })));
const PcDecorateEditor = lazy(() => import('./pages/PcDecorateEditor'));
const AiMcpSettings = lazy(() => import('./pages/AiMcpSettings'));
const AiAssistant = lazy(() => import('./pages/AiAssistant'));
const AiChannels = lazy(() => import('./pages/AiChannels'));
const AiUsage = lazy(() => import('./pages/AiUsage'));
const AiGuideSettings = lazy(() => import('./pages/AiGuideSettings'));
const OpsCodeSettings = lazy(() => import('./pages/OpsCodeSettings'));
const AiMcpApprovals = lazy(() => import('./pages/AiMcpApprovals'));
const AiMcpLogs = lazy(() => import('./pages/AiMcpLogs'));
const ShopDashboardManage = lazy(() => import('./pages/selfService').then((m) => ({ default: m.ShopDashboardManage })));
const ShopBalanceManage = lazy(() => import('./pages/selfService').then((m) => ({ default: m.ShopBalanceManage })));
const ShopAccountListManage = lazy(() => import('./pages/selfService').then((m) => ({ default: m.ShopAccountListManage })));
const ShopWithdrawManage = lazy(() => import('./pages/selfService').then((m) => ({ default: m.ShopWithdrawManage })));
const ShopDepositManage = lazy(() => import('./pages/selfService').then((m) => ({ default: m.ShopDepositManage })));
const ShopFinancialLogSelfManage = lazy(() => import('./pages/selfService').then((m) => ({ default: m.ShopFinancialLogSelfManage })));
const ShopProductCategoryManage = lazy(() => import('./pages/selfService').then((m) => ({ default: m.ShopProductCategoryManage })));
const VendorDashboardManage = lazy(() => import('./pages/selfService').then((m) => ({ default: m.VendorDashboardManage })));
const VendorBalanceManage = lazy(() => import('./pages/selfService').then((m) => ({ default: m.VendorBalanceManage })));
const VendorAccountListManage = lazy(() => import('./pages/selfService').then((m) => ({ default: m.VendorAccountListManage })));
const VendorWithdrawManage = lazy(() => import('./pages/selfService').then((m) => ({ default: m.VendorWithdrawManage })));
const VendorDepositManage = lazy(() => import('./pages/selfService').then((m) => ({ default: m.VendorDepositManage })));
const VendorInfoManage = lazy(() => import('./pages/selfService').then((m) => ({ default: m.VendorInfoManage })));
const VendorProductStockManage = lazy(() => import('./pages/selfService').then((m) => ({ default: m.VendorProductStockManage })));
const VendorStockLogManage = lazy(() => import('./pages/selfService').then((m) => ({ default: m.VendorStockLogManage })));
// 简版移动装修编辑器已彻底移除（原 ./pages/MobileDecorateEditor 文件已删除，2026-11-15）；
// ?page=mobileDecorateEditor 由下方 MobileDecorateEditorRedirect 跳转到新版 H5 装修编辑器。
const H5DecorateEditor = lazy(() => import('./pages/decorateEditorH5'));
const AccountPanel = lazy(() => import('./pages/FinanceOverviewPages').then((m) => ({ default: m.AccountPanel })));
const DistributionCommission = lazy(() => import('./pages/FinanceOverviewPages').then((m) => ({ default: m.DistributionCommission })));
const ShopWithdrawApplySummary = lazy(() => import('./pages/FinanceOverviewPages').then((m) => ({ default: m.ShopWithdrawApplySummary })));
const StaffFundWallet = lazy(() => import('./pages/FinanceOverviewPages').then((m) => ({ default: m.StaffFundWallet })));
const WechatMenu = lazy(() => import('./pages/WechatVoicePages').then((m) => ({ default: m.WechatMenu })));
const WechatKeywordReply = lazy(() => import('./pages/WechatVoicePages').then((m) => ({ default: m.WechatKeywordReply })));
const WechatAutoReply = lazy(() => import('./pages/WechatVoicePages').then((m) => ({ default: m.WechatAutoReply })));
const VoiceBroadcast = lazy(() => import('./pages/WechatVoicePages').then((m) => ({ default: m.VoiceBroadcast })));
const PromotionOverview = lazy(() => import('./pages/PromotionExtraPages').then((m) => ({ default: m.PromotionOverview })));
const PromotionStyle = lazy(() => import('./pages/PromotionExtraPages').then((m) => ({ default: m.PromotionStyle })));
const RewardGift = lazy(() => import('./pages/PromotionExtraPages').then((m) => ({ default: m.RewardGift })));
const SignInSetting = lazy(() => import('./pages/PromotionExtraPages').then((m) => ({ default: m.SignInSetting })));
const ECard = lazy(() => import('./pages/PromotionExtraPages').then((m) => ({ default: m.ECard })));
const Lottery = lazy(() => import('./pages/Lottery'));
const GiftCards = lazy(() => import('./pages/GiftCards'));
const PrintDesigner = lazy(() => import('./pages/PrintDesigner'));
const ThemeStyle = lazy(() => import('./pages/DecorateMobilePages').then((m) => ({ default: m.ThemeStyle })));
const MobileDecoratePages = lazy(() => import('./pages/DecorateMobilePages').then((m) => ({ default: m.MobileDecoratePages })));
const MobileSplashAd = lazy(() => import('./pages/DecorateMobilePages').then((m) => ({ default: m.MobileSplashAd })));
const MobileNav = lazy(() => import('./pages/DecorateMobilePages').then((m) => ({ default: m.MobileNav })));
const ClassifyDecorate = lazy(() => import('./pages/DecorateMobilePages').then((m) => ({ default: m.ClassifyDecorate })));
const MobileUserDecorate = lazy(() => import('./pages/DecorateMobilePages').then((m) => ({ default: m.MobileUserDecorate })));
const MobileUserOverseas = lazy(() => import('./pages/DecorateMobilePages').then((m) => ({ default: m.MobileUserOverseas })));
const MobileSearchStyle = lazy(() => import('./pages/DecorateMobilePages').then((m) => ({ default: m.MobileSearchStyle })));
const MobileCatNav = lazy(() => import('./pages/DecorateMobilePages').then((m) => ({ default: m.MobileCatNav })));
const PcDecorateHome = lazy(() => import('./pages/DecoratePcPages').then((m) => ({ default: m.PcDecorateHome })));
const PcNavigation = lazy(() => import('./pages/DecoratePcPages').then((m) => ({ default: m.PcNavigation })));
const PcCatFloor = lazy(() => import('./pages/DecoratePcPages').then((m) => ({ default: m.PcCatFloor })));
const PcCustomPage = lazy(() => import('./pages/DecoratePcPages').then((m) => ({ default: m.PcCustomPage })));
const PcProductStyle = lazy(() => import('./pages/DecoratePcPages').then((m) => ({ default: m.PcProductStyle })));
const PcSearchDecorate = lazy(() => import('./pages/DecoratePcPages').then((m) => ({ default: m.PcSearchDecorate })));
const PcLoginDecorate = lazy(() => import('./pages/DecoratePcPages').then((m) => ({ default: m.PcLoginDecorate })));
const PcSearchStyle = lazy(() => import('./pages/DecoratePcPages').then((m) => ({ default: m.PcSearchStyle })));
const PcDecorateOther = lazy(() => import('./pages/DecoratePcPages').then((m) => ({ default: m.PcDecorateOther })));
const ConsumerRanking = lazy(() => import('./pages/PanelStatisticsPages').then((m) => ({ default: m.ConsumerRanking })));
const SaleTargets = lazy(() => import('./pages/PanelStatisticsPages').then((m) => ({ default: m.SaleTargets })));
const StatisticsGoods = lazy(() => import('./pages/PanelStatisticsPages').then((m) => ({ default: m.StatisticsGoods })));
const StatisticsUser = lazy(() => import('./pages/PanelStatisticsPages').then((m) => ({ default: m.StatisticsUser })));
const NewMembers = lazy(() => import('./pages/PanelChartPages').then((m) => ({ default: m.NewMembers })));
const StatisticsAccess = lazy(() => import('./pages/PanelChartPages').then((m) => ({ default: m.StatisticsAccess })));
const StatisticsOrder = lazy(() => import('./pages/PanelChartPages').then((m) => ({ default: m.StatisticsOrder })));
const StatisticsSale = lazy(() => import('./pages/PanelChartPages').then((m) => ({ default: m.StatisticsSale })));
const StatementDetails = lazy(() => import('./pages/FinanceStatementPages').then((m) => ({ default: m.StatementDetails })));
const ShopWithdrawApplies = lazy(() => import('./pages/FinanceStatementPages').then((m) => ({ default: m.ShopWithdrawApplies })));
const VendorWithdrawApplies = lazy(() => import('./pages/FinanceStatementPages').then((m) => ({ default: m.VendorWithdrawApplies })));
const VendorStatementDownload = lazy(() => import('./pages/FinanceStatementPages').then((m) => ({ default: m.VendorStatementDownload })));
const VendorWithdrawSummary = lazy(() => import('./pages/FinanceStatementPages').then((m) => ({ default: m.VendorWithdrawSummary })));
const AdminMerchantSettings = lazy(() => import('./pages/AdminMerchantSettings'));
const StaffFundSettings = lazy(() => import('./pages/StaffFundSettings'));
const DeliverOrders = lazy(() => import('./pages/DeliverOrders'));
const AftersalesOrders = lazy(() => import('./pages/AftersalesOrders'));
const OrderExport = lazy(() => import('./pages/OrderExport'));
const ProxyShopping = lazy(() => import('./pages/ProxyShopping'));
const ProxyHall = lazy(() => import('./pages/ProxyHall'));
const Coupons = lazy(() => import('./pages/Coupons'));
const RechargeSettings = lazy(() => import('./pages/RechargeSettings'));
const RechargeGiftRecords = lazy(() => import('./pages/RechargeGiftRecords'));
const MarketingDistribution = lazy(() => import('./pages/MarketingDistribution'));
const PointsExchanges = lazy(() => import('./pages/PointsExchanges'));
const ProductGifts = lazy(() => import('./pages/ProductGifts'));
const Seckills = lazy(() => import('./pages/Seckills'));
const AuctionItems = lazy(() => import('./pages/AuctionItems'));
const AuctionSessions = lazy(() => import('./pages/AuctionSessions'));
const AuctionDeposits = lazy(() => import('./pages/AuctionDeposits'));
const AuctionBreaches = lazy(() => import('./pages/AuctionBreaches'));
const AuctionSettings = lazy(() => import('./pages/AuctionSettings'));
const TimeDiscounts = lazy(() => import('./pages/TimeDiscounts'));
const ProductPromotions = lazy(() => import('./pages/ProductPromotions'));
const Groupons = lazy(() => import('./pages/Groupons'));
const InventoryQuery = lazy(() => import('./pages/InventoryQuery'));
const ProductInventoryLogs = lazy(() => import('./pages/ProductInventoryLogs'));
const InventoryAdjustments = lazy(() => import('./pages/InventoryAdjustments'));
const StockRecords = lazy(() => import('./pages/StockRecords'));
const InventoryTransfers = lazy(() => import('./pages/InventoryTransfers'));
const Enquiries = lazy(() => import('./pages/Enquiries'));
const StockWarnings = lazy(() => import('./pages/StockWarnings'));
const UserCertifications = lazy(() => import('./pages/UserCertifications'));
const MemberTags = lazy(() => import('./pages/MemberTags'));
const Feedbacks = lazy(() => import('./pages/Feedbacks'));
const MessageLogs = lazy(() => import('./pages/MessageLogs'));
const MemberLevels = lazy(() => import('./pages/MemberLevels'));
const ShopMemberSettings = lazy(() => import('./pages/ShopMemberSettings'));
const ShopCustomers = lazy(() => import('./pages/ShopCustomers'));
const CustomerBlacklist = lazy(() => import('./pages/CustomerBlacklist'));
const UserAuthenticationSettings = lazy(() => import('./pages/UserAuthenticationSettings'));
const MemberLevelPros = lazy(() => import('./pages/MemberLevelPros'));
const ArticleCategories = lazy(() => import('./pages/ArticleCategories'));
const KefuList = lazy(() => import('./pages/KefuList'));
const KefuAccessSettings = lazy(() => import('./pages/KefuAccessSettings'));
const KefuQuickReplies = lazy(() => import('./pages/KefuQuickReplies'));
const KefuAccountInfo = lazy(() => import('./pages/KefuAccountInfo'));
const WechatLives = lazy(() => import('./pages/WechatLives'));
const WechatLiveGoods = lazy(() => import('./pages/WechatLiveGoods'));
const VideoNumbers = lazy(() => import('./pages/VideoNumbers'));
const Authorities = lazy(() => import('./pages/Authorities'));
const MaterialCenter = lazy(() => import('./pages/MaterialCenter'));
const SupplyChainPage = lazy(() => import('./pages/SupplyChainPages'));
const CreditOrders = lazy(() => import('./pages/CreditPages').then((m) => ({ default: m.CreditOrders })));
const CreditUsers = lazy(() => import('./pages/CreditPages').then((m) => ({ default: m.CreditUsers })));
const CreditSettlements = lazy(() => import('./pages/CreditPages').then((m) => ({ default: m.CreditSettlements })));
const ProductBatch = lazy(() => import('./pages/ProductBatch'));
const InventoryCalendars = lazy(() => import('./pages/InventoryCalendars'));
const ShopList = lazy(() => import('./pages/ShopList'));
const ShopLabelManage = lazy(() => import('./pages/ShopLabelManage'));
const SalesmanShopManage = lazy(() => import('./pages/SalesmanShopManage'));
const ShopMiniProgramManage = lazy(() => import('./pages/ShopMiniProgramManage'));
const MerchantApplies = lazy(() => import('./pages/MerchantApplies'));
const Merchants = lazy(() => import('./pages/Merchants'));
const InvestmentContent = lazy(() => import('./pages/InvestmentContent'));
const VendorList = lazy(() => import('./pages/VendorList'));
const VendorProducts = lazy(() => import('./pages/VendorProducts'));
const VendorOrders = lazy(() => import('./pages/VendorOrders'));
const Placeholder = lazy(() => import('./pages/Placeholder'));
import { pageTitles } from './config/adminMenu';
import { configSections } from './config/configSections';
import { configHelp } from './config/configHelp';
import { configActions } from './config/configActions';
import type { ConfigAction } from './pages/AutoConfigPage';

type ConfigPageDef = {
  title: string;
  getApi: string;
  saveApi: string;
  sections?: Array<{ title: string; fields: string[] }>;
  /** 严格分区：只展示 sections 字段（共用接口拆页时用，见 AutoConfigPage）。 */
  sectionsOnly?: boolean;
  rules?: Record<string, ConfigFieldRule>;
  fieldGroups?: ConfigFieldGroup[];
  controls?: Record<string, 'radio' | 'switch' | 'segmented' | 'color' | 'options' | 'multi' | 'select' | 'richtext' | 'textarea'>;
  options?: Record<string, Array<{ label: string; value: string | number; color?: string }>>;
  hiddenFields?: string[];
  fieldHelp?: Record<string, string>;
  labels?: Record<string, string>;
  renderers?: Record<string, (context: ConfigRendererContext) => ReactNode>;
  numberLimits?: Record<string, ConfigNumberLimit>;
  adjustValues?: (
    key: string,
    value: unknown,
    values: Record<string, unknown>,
  ) => Record<string, unknown> | void;
  validate?: (values: Record<string, unknown>) => string | null | undefined;
  /** 旧版配置页里的按钮型功能（密钥、批量更新、测试邮件等），由 AutoConfigPage 统一渲染。 */
  actions?: ConfigAction[];
  modern?: boolean;
};

/** 多选字段的值可能是数组，也可能是逗号分隔字符串（AutoConfigPage 的 multi 控件）。 */
function splitMultiValue(value: unknown): unknown[] {
  if (Array.isArray(value)) return value;
  return String(value ?? '')
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean);
}

const configPages: Record<string, ConfigPageDef> = {
  storeSettingsManage: {
    title: '门店设置',
    getApi: '/setting/config/storeSettings',
    saveApi: '/setting/config/saveStore',
    controls: {
      storePostAllocationStatus: 'radio',
      storeIndependentGoods: 'switch',
      shopProductNeedCheck: 'switch',
      storeAssignProductName: 'switch',
      storeAssignProductImage: 'switch',
      storeAssignProductPrice: 'radio',
      storeUseSoloProductStock: 'switch',
      storeUseTotalProductStock: 'switch',
      storeAssignDeliveryServiceFee: 'radio',
      storeAssignServerServiceFee: 'radio',
      storeCustomSubmitShippingType: 'radio',
      storeShowOtherCityStore: 'radio',
      regionIsolation: 'radio',
    },
    options: {
      storePostAllocationStatus: [
        { label: '上架', value: 1 },
        { label: '下架', value: 0 },
      ],
      storeAssignProductPrice: [
        { label: '使用总部/区域统一售价', value: 0 },
        { label: '使用门店独立售价', value: 1 },
      ],
      storeAssignDeliveryServiceFee: [
        { label: '使用总部统一配置', value: 0 },
        { label: '使用门店独立配置', value: 1 },
      ],
      storeAssignServerServiceFee: [
        { label: '使用总部统一配置', value: 0 },
        { label: '使用门店独立配置', value: 1 },
      ],
      storeCustomSubmitShippingType: [
        { label: '自提优先', value: 0 },
        { label: '配送优先', value: 1 },
        { label: '同城优先', value: 2 },
      ],
      storeShowOtherCityStore: [
        { label: '显示', value: 1 },
        { label: '不显示', value: 0 },
      ],
      regionIsolation: [
        { label: '开启', value: 1 },
        { label: '关闭', value: 0 },
      ],
    },
    rules: {
      shopProductNeedCheck: { visibleWhen: { field: 'storeIndependentGoods', equals: 1 } },
    },
    fieldHelp: {
      maxRecommendProductCount: '商城中新品、推荐列表显示的商品数量',
      maxSubAdministrator: '当前店铺可配置的最大子管理员数',
      storePostAllocationStatus: '在将商品分配到组织时，可预设分配成功后商品在组织的上下架状态。',
      storeIndependentGoods: '开启后允许门店自建商品，关闭后门店不可自行创建商品。',
      shopProductNeedCheck: '门店发布商品的时候是否需要审核才能显示商品。开启为默认审核通过。',
      storeAssignProductName: '勾选后，门店可修改总部/区域分配商品的名称，不同步至门店。',
      storeAssignProductImage: '勾选后，门店可修改总部/区域分配商品的主图和详情页。',
      storeAssignProductPrice: '用统一售价时，门店商品销售价将取总部/区域设置的统一售价，门店不可修改。',
      storeAssignDeliveryServiceFee: '用统一配置时，门店配送费将取总部统一配置，门店端不可修改。',
      storeAssignServerServiceFee: '用统一配置时，门店服务费将取总部统一配置，门店端不可修改。',
      storeCustomSubmitShippingType: '商品存在多种配送方式时客户下单默认优先采用的方式。',
      storeShowOtherCityStore: '是否在地区中展示其他城市的门店。',
      regionIsolation:
        '开启后，买家只能看到自己所在地区的门店，以及这些门店有售的商品（平台自营商品不受影响）；未填写地区的门店将不可见。开启后「显示地区其他门店」对门店列表与最近门店不再生效。默认关闭。',
    },
  },

  vendorSettingManage: {
    title: '供应商设置',
    getApi: '/setting/config/vendorSettings',
    saveApi: '/setting/config/saveVendor',
    sections: [
      {
        title: '基本设置',
        fields: ['vendorProductNeedCheck', 'vendorProductAuditShelfType', 'vendorMaxSubAdministrator'],
      },
      {
        title: '价格设置',
        fields: ['vendorSetPriceType', 'vendorSetPriceAutoValue'],
      },
    ],
    controls: {
      vendorProductNeedCheck: 'switch',
      vendorProductAuditShelfType: 'radio',
      vendorSetPriceType: 'radio',
    },
    options: {
      vendorProductAuditShelfType: [
        { label: '自动上架', value: 1 },
        { label: '手动上架', value: 2 },
      ],
      vendorSetPriceType: [
        { label: '按比例', value: 1 },
        { label: '按固定数值加价', value: 2 },
        { label: '最大售卖金额', value: 3 },
      ],
    },
    rules: {
      vendorSetPriceAutoValue: { visibleWhen: { field: 'vendorSetPriceType', in: [1, 2] } },
    },
    fieldHelp: {
      vendorProductNeedCheck: '供应商发布商品的时候是否需要审核才能显示商品。开启为默认审核通过。',
      vendorProductAuditShelfType: '平台审核供应商商品通过后：自动上架立即上架供平台或门店选品；手动上架则审核通过后由供应商自行上架。',
      vendorMaxSubAdministrator: '当前供应商可配置最大子管理员数量',
      vendorSetPriceType: '设价方式：按比例以供货价为基数自动加价；按固定数值加价；或最大售卖金额。',
      vendorSetPriceAutoValue: '自动加价数值：按比例时为百分比，按固定数值时为金额。',
    },
  },

  posOrderSetting: {
    title: '收银设置',
    getApi: '/setting/config/getCashierOrderSettings',
    saveApi: '/setting/config/saveCashierOrderSettings',
    controls: {
      autoPrintOnlineOrder: 'options',
      autoPrintOfflineOrder: 'options',
      defaultCollectionMethod: 'options',
    },
    options: {
      autoPrintOnlineOrder: [
        { label: '开启', value: 1 },
        { label: '关闭', value: 2 },
      ],
      autoPrintOfflineOrder: [
        { label: '开启', value: 1 },
        { label: '关闭', value: 2 },
      ],
      defaultCollectionMethod: [
        { label: '现金支付', value: 1 },
        { label: '扫码支付', value: 2 },
        { label: '余额支付', value: 3 },
        { label: '积分支付', value: 4 },
      ],
    },
    fieldHelp: {
      autoEraseZero: '开启后订单结算时自动向下抹除零头',
      autoDeductionPoints: '开启后若商品设置了可使用积分抵扣，在结算时可自动抵扣金额',
      autoPrintOnlineOrder: '开启前请先前往小票打印页面配置相关参数',
      autoPrintOfflineOrder: '开启前请先前往小票打印页面配置相关参数',
      defaultCollectionMethod: '进入结算页面的默认付款方式',
      autoAddCartAfterScan: '扫码枪扫描商品条码并回车后自动加入购物车',
      posProductSource: '请选择至少一个商品来源',
      shopCommonRemarks: '收银机常用备注，一行一条',
    },
  },
  baseBasicManage: {
    title: '基础信息',
    getApi: '/setting/config/basicSettings',
    saveApi: '/setting/config/saveBasic',
    labels: {
      shopCompany: '版权信息',
      poweredBy: '技术支持标识',
      shopLogo: '商城LOGO-黑底',
      lightShopLogo: '商城LOGO-白底',
      closeOrder: '关闭下单',
    },
    hiddenFields: [
      'shopCompanyTxt',
      'defaultCopyright',
      'poweredByLogo',
      'defaultTechSupport',
      'businessLicense',
      'businessLicenseType',
      'ediLicense',
    ],
    controls: {
      poweredByStatus: 'radio',
      shopRegClosed: 'radio',
      closeOrder: 'radio',
    },
    renderers: {
      shopCompany: (context) => <CopyrightConfigField {...context} />,
      poweredBy: (context) => <TechSupportConfigField {...context} />,
      businessLicenseEnabled: (context) => <BusinessLicenseConfigField {...context} />,
      shopName: (context) => (
        <MultilingualInput value={String(context.value ?? '')} onChange={(next) => context.update('shopName', next)} />
      ),
      kefuAddress: (context) => (
        <MultilingualInput value={String(context.value ?? '')} onChange={(next) => context.update('kefuAddress', next)} />
      ),
      shopIcpNo: (context) => (
        <MultilingualInput value={String(context.value ?? '')} onChange={(next) => context.update('shopIcpNo', next)} />
      ),
    },
  },
  baseProductManage: {
    title: '商品设置',
    getApi: '/setting/config/productSettings',
    saveApi: '/setting/config/saveProduct',
    labels: {
      dollarSign: '商品货币符',
      dollarSignCn: '商品货币中文',
      snPrefix: '商品货号前缀',
      customProductNameLength: '商品名称长度',
      marketPriceRate: '市场价换算',
      productEditStockModify: '商品编辑库存是否可修改',
      specImageMode: '规格图片',
      showSelledCount: '显示销量',
      showMarketprice: '显示市场价',
      isEnquiry: '是否开启询价功能',
      bulkPurchase: '商品规格显示批量采购',
      enableAttributeFilter: '显示筛选信息',
      productCommentEnabled: '是否开启商品评价',
      anonymousCommentEnabled: '是否开启匿名评价',
    },
    controls: {
      productRawImageDownloadEnabled: 'options',
      productRawImageDownloadChannels: 'multi',
      productRawImageDownloadScopes: 'multi',
      productEditStockModify: 'options',
      specImageMode: 'options',
      bulkPurchase: 'options',
      showSelledCount: 'options',
      showMarketprice: 'options',
      isEnquiry: 'options',
      enableAttributeFilter: 'options',
      productCommentEnabled: 'options',
      anonymousCommentEnabled: 'options',
    },
    options: {
      productRawImageDownloadEnabled: [
        { label: '开启', value: 1 },
        { label: '关闭', value: 0 },
      ],
      productRawImageDownloadChannels: [
        { label: 'PC网页', value: 'pc' },
        { label: '网页H5', value: 'h5' },
        { label: '微信小程序', value: 'miniProgram' },
        { label: 'APP', value: 'app' },
      ],
      productRawImageDownloadScopes: [
        { label: '商品主图', value: 'main' },
        { label: 'sku图', value: 'sku' },
        { label: '商品详情图', value: 'detail' },
      ],
      productEditStockModify: [
        { label: '是', value: 1 },
        { label: '否', value: 0 },
      ],
      specImageMode: [
        { label: '仅第一组规格可上传图片', value: 1 },
        { label: '全部规格可上传图片', value: 2 },
      ],
      bulkPurchase: [
        { label: '是', value: 1 },
        { label: '否', value: 0 },
      ],
      showSelledCount: [
        { label: '是', value: 1 },
        { label: '否', value: 0 },
      ],
      showMarketprice: [
        { label: '是', value: 1 },
        { label: '否', value: 0 },
      ],
      isEnquiry: [
        { label: '开启', value: 1 },
        { label: '关闭', value: 0 },
      ],
      enableAttributeFilter: [
        { label: '是', value: 1 },
        { label: '否', value: 0 },
      ],
      productCommentEnabled: [
        { label: '开启', value: 1 },
        { label: '关闭', value: 0 },
      ],
      anonymousCommentEnabled: [
        { label: '开启', value: 1 },
        { label: '关闭', value: 0 },
      ],
    },
    fieldHelp: {
      productRawImageDownloadEnabled: '开启后，用户可在用户端商品详情页打包下载商品图片原图压缩包',
      productRawImageDownloadChannels: '勾选后，对应渠道用户端将显示商品原图下载入口',
      productRawImageDownloadScopes: '勾选后，所有商品自动打包对应商品图片内容',
    },
    rules: {
      anonymousCommentEnabled: {
        disabledWhen: { field: 'productCommentEnabled', notEquals: 1 },
      },
    },
    numberLimits: {
      customProductNameLength: { min: 0, max: 750, precision: 0, step: 1 },
      marketPriceRate: { min: 0, precision: 1, step: 0.1 },
    },
    adjustValues: (key, next, values) => {
      if (
        key === 'productCommentEnabled' &&
        Number(next) !== 1 &&
        Number(values.anonymousCommentEnabled) !== 0
      ) {
        return { anonymousCommentEnabled: 0 };
      }
      return undefined;
    },
    validate: (values) => {
      if (Number(values.productRawImageDownloadEnabled) !== 1) return null;
      if (!splitMultiValue(values.productRawImageDownloadChannels).length) {
        return '开启原图下载时，请至少选择一个渠道';
      }
      if (!splitMultiValue(values.productRawImageDownloadScopes).length) {
        return '开启原图下载时，请至少选择一种打包范围';
      }
      return null;
    },
  },
  baseShoppingManage: {
    title: '交易设置',
    getApi: '/setting/config/shoppingSettings',
    saveApi: '/setting/config/saveShopping',
    labels: {
      childAreaNeedRegion: '运费模板地区设置机制',
      integralName: '积分自定义名称',
      integralScale: '积分换算比例',
      orderSendPointNode: '赠送积分节点',
      orderSendPoint: '赠送积分',
      orderCompleteSendPointDays: '订单完成赠送积分天数',
      integralPercentBaseAmount: '积分抵扣基数金额',
      integralPercent: '积分支付比例',
      commentSendPoint: '评论商品赠送积分',
      showSendPoint: '晒单赠送积分',
      useQiandaoPoint: '签到赠送积分',
      continuousSignIn: '连续签到',
      canInvoice: '是否能开发票',
      invoiceAdded: '是否支持增值税专用发票',
      invoiceApplyNode: '可开票节点',
      invoiceApplyDeadlineDays: '发票申请期限（天）',
      returnConsignee: '回寄联系人设置',
      returnMobile: '回寄电话设置',
      returnAddress: '回寄地址设置',
    },
    controls: {
      orderSendPointNode: 'select',
      invoiceApplyNode: 'select',
    },
    options: {
      orderSendPointNode: [
        { label: '待发货', value: 1 },
        { label: '已发货', value: 2 },
        { label: '已完成', value: 3 },
      ],
      invoiceApplyNode: [
        { label: '不限制', value: 0 },
        { label: '支付后', value: 1 },
        { label: '确认收货后', value: 2 },
        { label: '订单完成后', value: 3 },
      ],
    },
    fieldHelp: {
      orderSendPointNode: '选择下单赠送积分的节点',
      invoiceApplyNode: '买家在订单达到该节点后才可申请发票，默认订单完成后',
      invoiceApplyDeadlineDays: '节点达成后多少天内可申请发票，0 表示不限制期限',
      orderCompleteSendPointDays: '选择已完成节点时，订单完成多少天后赠送积分',
      integralPercentBaseAmount: '每消费该金额最多可使用下方比例的积分',
    },
  },
  baseOrderManage: {
    title: '订单设置',
    getApi: '/setting/config/orderSettings',
    saveApi: '/setting/config/saveOrder',
    rules: {
      autoReturnGoods: { visibleWhen: { field: 'afterSalesReview', equals: 1 } },
      autoReturnGoodsDays: { disabledWhen: { field: 'autoReturnGoods', equals: 0 } },
    },
    controls: {
      afterSalesReasonEnabled: 'switch',
      returnRestockEnabled: 'switch',
      proxyCrossShopEnabled: 'switch',
      proxyHallEnabled: 'switch',
      proxyHallFallback: 'switch',
    },
    fieldHelp: {
      returnRestockEnabled: '开启后，退货退款在商家确认收货时按退回数量回补商品库存；默认关闭（不回补）',
      proxyCrossShopEnabled: '开启后，代客下单可选择同体系的其它门店履约，订单按履约门店价格计价（店员不可改价），订单完成后按下方三方比例分成',
      proxyHallEnabled: '开启后（需同时开启跨店履约），代客下单可选择「接单大厅」方式：订单进待接单池，由同体系门店抢单/派单履约；大厅单按下单店价格计价',
      proxyHallTimeoutMinutes: '接单大厅订单的接单时限（分钟），超时无人接单按下方兜底策略处置',
      proxyHallFallback: '开启 = 超时自动改派下单店自履约；关闭 = 超时后继续留在大厅等待接单',
      proxyPlatformRate: '三方合计不得超过 100%；比例在下单时快照，改比例只影响新订单',
      proxyShopRate: '代客下单店铺在订单完成结算时的分成比例',
      proxyFulfillRate: '履约门店分成比例（保底：未分配残值与舍入差归履约门店）',
    },
    renderers: {
      afterSalesReasonList: (context) => (
        <AfterSalesReasonField
          value={context.value}
          onChange={(next) => context.update('afterSalesReasonList', next)}
        />
      ),
    },
    validate: (values) => {
      // 代客跨店三方分成：任一比例非法或合计 >100% 拒绝保存（后端同口径强校验）
      const rateKeys = ['proxyPlatformRate', 'proxyShopRate', 'proxyFulfillRate'];
      if (rateKeys.some((k) => values[k] !== undefined && values[k] !== null && values[k] !== '')) {
        const rates = rateKeys.map((k) => Number(values[k]));
        if (rates.some((r) => !Number.isFinite(r) || r < 0 || r > 100)) {
          return '代客下单跨店分成比例须在 0~100 之间';
        }
        if (rates.reduce((a, b) => a + b, 0) > 100) {
          return '代客下单跨店分成比例合计不能超过 100%';
        }
      }
      if (Number(values.afterSalesReasonEnabled) !== 1) return null;
      if (!Array.isArray(values.afterSalesReasonList) || values.afterSalesReasonList.length === 0) {
        return '开启售后原因功能后，请至少添加一项原因';
      }
      return null;
    },
  },
  kefuSetting: { title: '客服设置', getApi: '/setting/config/kefuSettings', saveApi: '/setting/config/saveKefu' },
  globalSetting: {
    title: '全局设置',
    getApi: '/setting/config/globalSettings',
    saveApi: '/setting/config/saveGlobal',
    // SEO 相关字段（SEO设置/链接设置两区）已归拢到「设置 → SEO → SEO设置」
    // (seoSettings, 与本页共用 globalSettings 接口)：严格分区后本页不再展示
    // 它们，保存仍回写接口全量值，未展示字段不丢失。
    sectionsOnly: true,
    controls: {
      navTheme: 'segmented',
      primaryColor: 'color',
      layout: 'segmented',
      versionInfoHidden: 'radio',
      storageType: 'options',
      // 以下 3 项旧版是 radio，此前未登记 controls/options，静默退化成了文本框
      materialUploadNameType: 'radio',
      selectSearchType: 'radio',
      langType: 'radio',
    },
    options: {
      navTheme: [
        { label: '浅色', value: 'light' },
        { label: '深色', value: 'dark' },
      ],
      primaryColor: [
        { label: '蓝色', value: 'blue', color: '#1456f0' },
        { label: '红色', value: 'red', color: '#f5222d' },
        { label: '橙色', value: 'orange', color: '#faad14' },
        { label: '绿色', value: 'green', color: '#52c41a' },
        { label: '紫色', value: 'purple', color: '#722ed1' },
      ],
      layout: [
        { label: '默认', value: 'default' },
        { label: '侧边菜单', value: 'side' },
        { label: '顶部菜单', value: 'top' },
        { label: '混合菜单', value: 'mix' },
      ],
      storageType: [
        { label: '本地存储', value: 0 },
        { label: '阿里云 OSS', value: 1 },
        { label: '腾讯云 COS', value: 2 },
        { label: '七牛云 KODO', value: 3 },
        { label: 'AWS S3', value: 4 },
      ],
      materialUploadNameType: [
        { label: '系统随机命名', value: 0 },
        { label: '保留原素材名', value: 1 },
      ],
      selectSearchType: [
        { label: 'Elasticsearch（推荐）', value: 'es' },
        { label: 'MySQL（降级模式）', value: 'mysql' },
      ],
      langType: [{ label: '火山翻译', value: 1 }],
    },
    labels: {
      langType: '接口翻译',
    },
    hiddenFields: ['storageSaveFullPath'],
    fieldHelp: {
      storageType: '开启云存储后，图片会直接通过接口上传至指定云存储，不会再保存在本地。',
      storageLocalUrl: '参考格式： https://oss.***.com/ ，域名结尾必须带斜杠 /',
      storageOssUrl: '参考格式： https://oss.***.com/ ，域名结尾必须带斜杠 /',
      storageCosUrl: '参考格式： https://oss.***.com/ ，域名结尾必须带斜杠 /',
      storageQiniuUrl: '参考格式： https://oss.***.com/ ，域名结尾必须带斜杠 /',
      storageS3Url: '参考格式： https://oss.***.com/ ，Bucket URL 结尾必须带斜杠 /',
      storageOssAccessKeyId: '编辑完成后，点击下方提交按钮确认修改，请确保填写正确的 AccessKeyId',
      storageOssAccessKeySecret: '编辑完成后，点击下方提交按钮确认修改，请确保填写正确的 AccessKeySecret',
      storageCosSecretId: '编辑完成后，点击下方提交按钮确认修改，请确保填写正确的 SecretId',
      storageCosSecretKey: '编辑完成后，点击下方提交按钮确认修改，请确保填写正确的 KeySecret',
      storageQiniuAccessKey: '编辑完成后，点击下方提交按钮确认修改，请确保填写正确的 AccessKey',
      storageQiniuSecretKey: '编辑完成后，点击下方提交按钮确认修改，请确保填写正确的 SecretKey',
      storageS3AccessKey: '编辑完成后，点击下方提交按钮确认修改，请确保填写正确的 Access key ID',
      storageS3SecretKey: '编辑完成后，点击下方提交按钮确认修改，请确保填写正确的 Secret access key',
      selectSearchType: '选择 Elasticsearch 后，点击「同步到搜索引擎」将一次性把所有商品数据同步到搜索引擎，异步执行预计需要几分钟，请勿重复点击。',
    },
    renderers: {
      defaultCountry: (context) => <CountrySelectField {...context} />,
      shopDefaultRegions: (context) => <ShopRegionCascaderField {...context} />,
    },
    rules: {
      storageLocalUrl: { visibleWhen: { field: 'storageType', equals: 0 } },
      storageOssUrl: { visibleWhen: { field: 'storageType', equals: 1 } },
      storageOssAccessKeyId: { visibleWhen: { field: 'storageType', equals: 1 } },
      storageOssAccessKeySecret: { visibleWhen: { field: 'storageType', equals: 1 } },
      storageOssBucket: { visibleWhen: { field: 'storageType', equals: 1 } },
      storageOssRegion: { visibleWhen: { field: 'storageType', equals: 1 } },
      storageCosUrl: { visibleWhen: { field: 'storageType', equals: 2 } },
      storageCosSecretId: { visibleWhen: { field: 'storageType', equals: 2 } },
      storageCosSecretKey: { visibleWhen: { field: 'storageType', equals: 2 } },
      storageCosBucket: { visibleWhen: { field: 'storageType', equals: 2 } },
      storageCosRegion: { visibleWhen: { field: 'storageType', equals: 2 } },
      storageQiniuUrl: { visibleWhen: { field: 'storageType', equals: 3 } },
      storageQiniuAccessKey: { visibleWhen: { field: 'storageType', equals: 3 } },
      storageQiniuSecretKey: { visibleWhen: { field: 'storageType', equals: 3 } },
      storageQiniuBucket: { visibleWhen: { field: 'storageType', equals: 3 } },
      storageQiniuRegion: { visibleWhen: { field: 'storageType', equals: 3 } },
      storageS3Url: { visibleWhen: { field: 'storageType', equals: 4 } },
      storageS3AccessKey: { visibleWhen: { field: 'storageType', equals: 4 } },
      storageS3SecretKey: { visibleWhen: { field: 'storageType', equals: 4 } },
      storageS3Bucket: { visibleWhen: { field: 'storageType', equals: 4 } },
      storageS3Region: { visibleWhen: { field: 'storageType', equals: 4 } },
      storageS3Endpoint: { visibleWhen: { field: 'storageType', equals: 4 } },
    },
  },
  // SEO 设置（我方新增二级菜单「SEO」归拢）：与 globalSetting 共用
  // globalSettings/saveGlobal 接口与同一份配置数据，仅按主题拆页展示；
  // sectionsOnly 保证本页只显示 SEO设置/链接设置两区，保存回写全量值。
  seoSettings: {
    title: 'SEO设置',
    getApi: '/setting/config/globalSettings',
    saveApi: '/setting/config/saveGlobal',
    sectionsOnly: true,
    controls: {
      seoCategoryEnable: 'radio',
      seoProductEnable: 'radio',
    },
    options: {
      seoCategoryEnable: [
        { label: '基础链接', value: 0 },
        { label: 'SEO链接', value: 1 },
      ],
      seoProductEnable: [
        { label: '基础链接', value: 0 },
        { label: 'SEO链接', value: 1 },
      ],
    },
    labels: {
      seoCategoryEnable: '商品类目链接使用',
      seoProductEnable: '商品链接使用',
    },
    fieldHelp: {
      seoCategoryEnable: '链接格式：域名/category/分类1/分类2；「批量更新商品类目链接」仅更新 SEO 链接为空的类目。',
      seoProductEnable: '链接格式：域名/product/分类1/分类2/商品英文名称.[sn]；「批量更新商品链接」仅更新 SEO 链接为空的商品。',
    },
  },
  loginSetting: {
    title: '登录设置',
    getApi: '/setting/config/loginSettings',
    saveApi: '/setting/config/saveLogin',
    controls: {
      openWechatOauth: 'radio',
      openWechatPcLogin: 'radio',
      wechatRegisterBindPhone: 'radio',
      openWechatRegister: 'radio',
      wechatShareNeedLogin: 'radio',
      wechatH5MobileLogin: 'radio',
      wechatH5PasswordLogin: 'radio',
      facebookLoginOn: 'radio',
      googleLoginOn: 'radio',
      twitterLoginOn: 'radio',
    },
    rules: {
      openWechatPcLogin: {
        disabledWhen: { field: 'openWechatOauth', equals: 0 },
      },
      wechatRegisterBindPhone: {
        disabledWhen: {
          any: [
            { field: 'openWechatOauth', equals: 0 },
            { field: 'openWechatPcLogin', equals: 0 },
          ],
        },
      },
      openWechatRegister: {
        disabledWhen: { field: 'openWechatOauth', equals: 0 },
      },
      facebookClientId: { visibleWhen: { field: 'facebookLoginOn', equals: 1 } },
      facebookClientSecret: { visibleWhen: { field: 'facebookLoginOn', equals: 1 } },
      googleClientId: { visibleWhen: { field: 'googleLoginOn', equals: 1 } },
      googleClientSecret: { visibleWhen: { field: 'googleLoginOn', equals: 1 } },
      twitterClientId: { visibleWhen: { field: 'twitterLoginOn', equals: 1 } },
      twitterClientSecret: { visibleWhen: { field: 'twitterLoginOn', equals: 1 } },
    },
    fieldGroups: [
      {
        title: '公众号授权配置',
        description: '配置微信公众号授权登录所需的 AppID、密钥与服务器校验信息。',
        tag: '微信',
        icon: '微',
        fields: ['wechatAppId', 'wechatAppSecret', 'wechatServerToken', 'wechatServerSecret'],
        visibleWhen: { field: 'openWechatOauth', equals: 1 },
      },
    ],
  },
  storeAmapSetting: { title: '地图设置', getApi: '/setting/config/mapSettings', saveApi: '/setting/config/saveMapSettings' },
  personalizedManage: { title: '个性化设置', getApi: '/setting/config/getPersonalizedSetting', saveApi: '/setting/config/savePersonalizedSetting' },
};

Object.entries(configPages).forEach(([key, page]) => {
  page.sections = configSections[key];
  page.fieldHelp = { ...configHelp[key], ...page.fieldHelp };
  page.actions = configActions[key];
});

const knownPages = new Set(['dashboard', 'pcDecorateEditor', 'imWorkbench', 'kefuH5', 'mobileDecorateEditor', 'h5DecorateEditor', 'shopGroupManage', 'shopTipManage', 'offlinePaymentClaimManage', 'imageSearchManage', 'vendorAccountManage', 'vendorFundsManage', 'vendorFinancialLogManage', 'products', 'brandManage', 'categoryManage', 'productGroupManage', 'productServicesManage', 'commentManage', 'productAttributesTplManage', 'friendLinksManage', 'regionManage', 'logisticsCompanyManage', 'appVersionManage', 'clientReleaseManage', 'systemUpdate', 'adminLogManage', 'adminUserManage', 'adminRoleManage', 'serviceAgreement', 'privacyAgreement', 'afterSaleService', 'accountManage', 'taxRateManage', 'shippingTypeManage', 'logisticsManage', 'logisticsCompanyConfigManage', 'payLogManage', 'refundLogManage', 'userBalanceLogManage', 'staffFundWalletLogManage', 'buyerShowManage', 'integralLogManage', 'refundApplyManage', 'userWithdrawApplyManage', 'userRechargeOrderManage', 'userInvoiceManage', 'orderInvoiceManage', 'depositRuleManage', 'depositPayeeAccountManage', 'depositAccountManage', 'depositPaymentAuditManage', 'depositLogManage', 'categoryFeeRuleManage', 'merchantApplicationManage', 'subMerchantManage', 'profitSharingSettlementManage', 'statementDownload', 'pos0', 'posOrderSetting', 'posCashierDeviceList', 'posReceiptPrint', 'posCashierList', 'baseBasicManage', 'baseProductManage', 'baseShoppingManage', 'baseOrderManage', 'kefuSetting', 'globalSetting', 'seoSettings', 'loginSetting', 'storeAmapSetting', 'personalizedManage', 'baseNoticeManage', 'paymentSetting', 'withdrawalSettingManage', 'profitsharingSettingManage', 'depositSettingManage', 'priceDisplaySettings', 'paidCancelRefundSettings', 'giftOrderSettings', 'buyStyleSettings', 'systemMode', 'orgOverview', 'hubLinks', 'hubPairing', 'myLinks', 'hubMembers', 'hubOrders', 'hubSso', 'hubConfig', 'hubCredentials', 'hubAggregation', 'feedKeyManage', 'federationSettings', 'siteClone', 'fontSettings', 'shopSettingsManage', 'adminMerchantSettingsManage', 'deliveryFundSettingsManage', 'serverFundSettingsManage', 'deliverManage', 'aftersalesManage', 'orderExportManage', 'exchangeOrderManage', 'cycleBuyManage', 'proxyShoppingManage', 'proxyHallManage', 'couponManage', 'rechargeSettingManage', 'rechargeGiftRecordManage', 'marketingDistributionManage', 'pointsExchangeManage', 'productGiftManage', 'seckillManage', 'auctionItemManage', 'auctionSessionManage', 'auctionDepositManage', 'auctionBreachManage', 'auctionSettings', 'limitdiscountManage', 'fullReductionManage', 'grouponPromotionManage', 'inventoryQueryManage', 'productInventoryLogManage', 'inventoryAdjustmentManage', 'inventoryGoodsArePutIntoStorageManage', 'inventoryGoodsArePutOutOfStorageManage', 'storeInventoryTransferManage', 'enquiryManage', 'stockWarningManage', 'productBatchExportManage', 'productBatchAddManage', 'productBatchEditManage', 'storeInventoryCalendarManage', 'memberTagManage', 'feedbackManage', 'messageLogManage', 'levelManageManage', 'shopMemberSettingManage', 'shopMemberCustomerManage', 'userCertificationManage', 'userAuthenticationManage', 'userLevelProManage', 'articleCategoryManage', 'kefuManageList', 'kefuAccessSetting', 'kefuQuickReply', 'kefuAccountInfo', 'wechatLive', 'wechatLiveProduct', 'videoNumber', 'baseReceiptManage', 'authorityManage', 'materialCenter', 'termOrderManage', 'termPayManage', 'termMemberManage', 'shopManage', 'shopLabelManage', 'shopMiniProgramManage', 'applyManage', 'adminMerchantManage', 'investmentManage', 'vendorListManage', 'vendorProductManage', 'vendorOrderManage', 'vendorSettingManage', 'storeSettingsManage', 'storeTipManage', 'storeAreaManage', 'storeListManage', 'storeSelfPackListManage', 'deliverymanList', 'servicemanList', 'staffApplyManage', 'deliveryOrderManage', 'statementDetails', 'shopWithdrawApplyManage', 'vendorWithdrawApplyManage', 'vendorStatementDownload', 'vendorWithdrawApplySummary', 'deliveryStaffWithdrawManage', 'serverStaffWithdrawManage', 'wholesalerManage', 'wholesalerGradeManage', 'wholesaleSettingManage', 'wholesaleProductManage', 'wholesaleSummaryManage', 'storePickupSettingManage', 'storeAppointmentTplManage', 'storeEntryRuleSetting', 'wechatLiveMiniAppSetting', 'shippingTplManage', 'messageTypeManage', 'consumerRanking', 'saleTargets', 'statisticsGoods', 'statisticsUser', 'newMembers', 'statisticsAccess', 'statisticsOrder', 'statisticsSale', 'salesman0', 'overviewManage', 'AnalysisTableManage', 'analysisTable', 'RankingTableManage', 'rankingTable', 'subgroupManage', 'subgroup', 'salesmanNoticeManage', 'salesmanNotice', 'materialCategoryManage', 'materialCategory', 'materialManage', 'material', 'salesmanDetailedManage', 'salesmanDetailed', 'customerTransactionsManage', 'customerTransactions', 'performanceSettlementSettingManage', 'performanceSettlementSetting', 'performanceSettlementManage', 'performanceSettlement', 'promoteModeManage', 'promoteMode', 'salesmanProductManage', 'salesmanProduct', 'promoterManage', 'promoter', 'salesmanShopManage', 'languagesList', 'currencyManagement', 'translationContent', 'verbalAssociation', 'areaCode', 'aiImageSetting', 'intraCityDeliveryManage', 'intraCityDelivery', 'homeServiceManage', 'homeService', 'premiumMemberManage', 'premiumMember', 'premiumMemberRightManage', 'premiumMemberRight', 'supplierManage', 'supplierCategoryManage', 'supplierProductManage', 'purchaseSettingManage', 'purchaseOrderManage', 'purchaseInboundManage', 'purchaseReturnManage', 'purchaseReconciliationManage', 'analysisTableManage', 'rankingTableManage', 'languagesListManage', 'aiImageSettingManage', 'translationContentManage', 'currencyManagementManage', 'areaCodeManage', 'verbalAssociationManage', 'videoNumber', 'selfVendorManage', 'selfVendorCategoryManage', 'selfVendorProductManage', 'purchaseReceiptManage', 'accountPanelManage', 'distributionCommissionManage', 'shopWithdrawApplySummary', 'vendorStatementDetails', 'shopAccountManage', 'shopFundsManage', 'financialLogManage', 'licensed', 'copyright', 'aiAssistantManage', 'aiAssistant', 'aiChannels', 'aiUsage', 'aiGuideSettings', 'opsCodeSettings', 'aiMcpManage', 'aiMcpSettings', 'aiMcpApprovals', 'aiMcpLogs', 'customerBlacklistManage', 'kitchenBoard', 'riderTrackManage', 'storeMapManage', 'platformAnnouncement', 'takeoutManage',
  // 角色自助页面集（商户端/供应商端，selfServiceMenu.tsx）
  'dashboardManage', 'balanceManage', 'withdrawManage', 'accountListManage', 'shopDepositManage', 'vendorDepositManage', 'merchantApplyManage', 'shopProductCategoryManage', 'vendorInfoManage', 'vendorEmployeeManage', 'vendorProductStockManage', 'vendorStockLogManage', 'printDesignerManage', 'adPromotionManage', 'adCampaignManage', 'adSettingsManage',
'EmployeeManagement', 'vendorUserManage', 'bindShopManager', 'ShopTeamManage', 'ShopInfoManage', 'EmployeeLogManagement', 'mobileShopDecorate', 'staffFundWalletManage', 'wechatMenu', 'wechatKeywordReply', 'wechatAutoReply', 'voiceBroadcastManage', 'promotion0', 'promotion2', 'rewardGiftManage', 'signInSettingManage', 'eCardManage', 'giftCardManage', 'lotteryManage', 'decorate0', 'mobileDecorateManage', 'mobileSplashAdManage', 'mobileSplashAdManage2', 'mobileuserUserOverseasManage', 'mobileSearchStyleManage', 'mobileNavManage', 'classifyManage', 'mobileUserManage', 'mobileCatNavManage', 'pcDecorateManage', 'pcNavigationManage', 'pcCatFloorManage', 'pcDecorateCustomManage', 'pcProductStyleManage','pcSearchStyleManage','pcDecorateOtherManage', 'pcSearchDecorateManage', 'pcLoginDecorateManage', 'customerService0', 'kefuHistoryList', 'afterSalesServiceManage', 'orders', 'members', 'content', 'decorate', 'decorate1', 'decorate2', 'pcDecorate', 'pcDecorateEdit', 'pcCustomPage']);

if (new URLSearchParams(window.location.search).get('demo') === '1') {
  enterDemo();
}

function normalizePageKey(key: string | null): PageKey {
  if (!key) return 'dashboard';
  const aliasMap: Record<string, string> = {
    decorate: 'decorate0',
    decorate0: 'decorate0',
    decorate1: 'mobileDecorateManage',
    mobileDecorate: 'mobileDecorateManage',
    mobileDecorateEdit: 'mobileDecorateManage',
    decorate2: 'pcDecorateManage',
    pcDecorate: 'pcDecorateManage',
    pcDecorateManage: 'pcDecorateManage',
    pcDecorateEdit: 'pcDecorateManage',
    pcCustomPage: 'pcDecorateCustomManage',
    pcCustomPageDecorateEdit: 'pcDecorateCustomManage',
    mobileShopDecorate: 'mobileDecorateManage',
  };
  if (aliasMap[key]) return aliasMap[key];
  return pageTitles[key] ? key : 'dashboard';
}

function initialPage(): string {
  const urlParams = new URLSearchParams(window.location.search);
  const pageParam = urlParams.get('page');
  if (pageParam === 'pcDecorateEditor' || pageParam === 'mobileDecorateEditor' || pageParam === 'h5DecorateEditor' || pageParam === 'imWorkbench' || pageParam === 'kefuH5') {
    return pageParam;
  }
  if (pageParam) {
    return normalizePageKey(pageParam);
  }
  const pathname = window.location.pathname;
  if (pathname.includes('/decorate/pc')) {
    return 'pcDecorateManage';
  }
  if (pathname.includes('/decorate/index')) {
    return 'mobileDecorateManage';
  }
  return 'dashboard';
}

/**
 * 简版移动装修编辑器下线跳转（2026-11-14；旧组件文件已于 2026-11-15 彻底删除）：
 * 旧简版编辑器只有 27 个模块 type 且全是通用占位预览（连 product/coupon/whiteBlank 都缺），
 * 已由新版 H5 装修编辑器（decorateEditorH5）全面替代。此组件把 ?page=mobileDecorateEditor 的
 * 全部查询参数（id / pageType / from 等）原样透传到 ?page=h5DecorateEditor，避免旧书签、
 * DecorateMobilePages 的默认 editPage 链接落到残缺编辑器。
 */
function MobileDecorateEditorRedirect() {
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    params.set('page', 'h5DecorateEditor');
    window.location.replace(`${window.location.origin}/?${params.toString()}`);
  }, []);
  // 不渲染任何中转提示（2026-11-18 用户裁示）：此前会闪现「旧版移动装修编辑器
  // 已下线，正在跳转…」整页文字、误导用户以为进错了页面；现静默直达新版编辑器。
  return null;
}


/** 懒加载页面 chunk 加载中的轻量占位（与后台浅灰背景一致，不引入额外依赖）。 */
function PageLoadingFallback() {
  return (
    <div style={{ padding: '64px 0', textAlign: 'center', color: 'var(--text-secondary, #475467)', fontSize: 14 }}>
      加载中…
    </div>
  );
}

export default function App() {
  const [authed, setAuthed] = useState(() => Boolean(getToken() || isDemo()));
  const [page, setPage] = useState<string>(initialPage);
  const [appearance, setAppearance] = useState<AppearanceSettings>({
    navTheme: 'light',
    primaryColor: 'blue',
    layout: 'default',
  });
  const [mallSettings, setMallSettings] = useState<MallSettings | null>(null);

  const applyAndStore = (values: AppearanceSettings) => {
    const next = {
      navTheme: values.navTheme || 'light',
      primaryColor: values.primaryColor || 'blue',
      layout: values.layout || 'default',
      adminLightLogo: values.adminLightLogo || '',
    };
    setAppearance(next);
    applyAppearance(next);
    saveAppearance(next);
  };

  const applyGlobalSettings = (values: Record<string, unknown>) => {
    applyAndStore(values as unknown as AppearanceSettings);
    setMallSettings(values as MallSettings);
    cacheMallSettings(values as MallSettings);
  };

  useEffect(() => {
    if (!authed) return;
    apiGet<Record<string, unknown>>('/setting/config/globalSettings')
      .then((result) => applyGlobalSettings(result || {}))
      .catch(() => applyAppearance({}));
  }, [authed]);

  // 网站字体：登录后拉取字体配置并应用后台字体（失败时保持系统默认字体）
  useEffect(() => {
    if (!authed) return;
    apiGet<FontSettingsConfig>('/setting/config/fontSettings')
      .then((result) => {
        if (result && typeof result === 'object') applyAdminFont(result);
      })
      .catch(() => undefined);
  }, [authed]);

  const changePage = (next: PageKey) => {
    const target = normalizePageKey(next);
    setPage(target);
    const url = new URL(window.location.href);
    url.searchParams.set('page', target);
    window.history.replaceState(null, '', url);
  };

  // 跨页跳转事件（列表页携带筛选参数跳转，如库存预警 → 商品列表）
  useEffect(() => {
    const handler = (event: Event) => {
      const detail = (event as CustomEvent<{ page: string; params?: Record<string, string> }>).detail;
      if (!detail?.page) return;
      const url = new URL(window.location.href);
      Object.entries(detail.params || {}).forEach(([key, value]) => url.searchParams.set(key, value));
      window.history.replaceState(null, '', url);
      changePage(detail.page);
    };
    window.addEventListener('admin-navigate', handler);
    return () => window.removeEventListener('admin-navigate', handler);
  }, []);

  if (!authed) {
    return (
      <Suspense fallback={<PageLoadingFallback />}>
        <Login onSuccess={() => setAuthed(true)} />
      </Suspense>
    );
  }

  // 客服工作台：独立全屏页（对齐旧版 /im/index 专属布局，菜单新标签打开）
  if (page === 'imWorkbench') {
    return (
      <Suspense fallback={<PageLoadingFallback />}>
        <KefuWorkbenchStandalone />
      </Suspense>
    );
  }

  // 客服工作台 H5 移动端（帖 477700）：独立全屏页，手机浏览器登录后直接接待，
  // 与 PC 版共用 root/im 接口与客服账号权限，不另造消息通道。
  if (page === 'kefuH5') {
    return (
      <Suspense fallback={<PageLoadingFallback />}>
        <KefuH5Workbench />
      </Suspense>
    );
  }

  // H5 装修编辑器（NutUI-React 重构版）：独立全屏页
  if (page === 'h5DecorateEditor') {
    const h5EditorId = Number(new URLSearchParams(window.location.search).get('id') || 0);
    const h5PageType = Number(new URLSearchParams(window.location.search).get('pageType') || 1);
    return (
      <Suspense fallback={<PageLoadingFallback />}>
      <H5DecorateEditor
        decorateId={h5EditorId}
        pageType={h5PageType}
        onExit={() => {
          window.location.href = `${window.location.origin}/?page=mobileDecorateManage`;
        }}
      />
      </Suspense>
    );
  }

  // 移动装修编辑器（旧简版 MobileDecorateEditor 已下线）：统一跳转新版 H5 装修编辑器，
  // 查询参数（id/pageType/from）原样透传，旧链接/书签不失效。旧组件文件保留不删。
  if (page === 'mobileDecorateEditor') {
    return <MobileDecorateEditorRedirect />;
  }

  if (page === 'pcDecorateEditor') {
    const editorId = Number(new URLSearchParams(window.location.search).get('id') || 0);
    return (
      <Suspense fallback={<PageLoadingFallback />}>
      <PcDecorateEditor
        decorateId={editorId}
        onExit={() => {
          const from = new URLSearchParams(window.location.search).get('from') || 'pcDecorateManage';
          window.location.href = `${window.location.origin}/?page=${from}`;
        }}
      />
      </Suspense>
    );
  }

  return (
    <AdminLayout
      layout={appearance.layout}
      logo={appearance.adminLightLogo}
      mallUrl={resolveMallUrl(mallSettings)}
      page={page as PageKey}
      onPageChange={changePage}
      onLogout={() => {
        clearToken();
        setAuthed(false);
      }}
    >
      <Suspense fallback={<PageLoadingFallback />}>
      {page === 'aiAssistantManage' && <AiAssistant />}
      {page === 'aiAssistant' && <AiAssistant />}
      {page === 'aiChannels' && <AiChannels />}
      {page === 'aiUsage' && <AiUsage />}
      {page === 'aiGuideSettings' && <AiGuideSettings />}
      {page === 'opsCodeSettings' && <OpsCodeSettings />}
      {page === 'aiMcpManage' && <AiMcpSettings />}
      {page === 'aiMcpSettings' && <AiMcpSettings />}
      {page === 'aiMcpApprovals' && <AiMcpApprovals />}
      {page === 'aiMcpLogs' && <AiMcpLogs />}
      {page === 'dashboard' && <Dashboard onNavigate={changePage} />}
      {page === 'products' && <Products />}
      {page === 'brandManage' && <Brands />}
      {page === 'productGroupManage' && <ProductGroups />}
      {page === 'categoryManage' && <Categories />}
      {page === 'productServicesManage' && <ProductServices />}
      {page === 'commentManage' && <Comments />}
      {page === 'productAttributesTplManage' && <AttributesTemplates />}
      {page === 'friendLinksManage' && <FriendLinks />}
      {page === 'regionManage' && <Regions />}
      {page === 'logisticsCompanyManage' && <LogisticsCompanies />}
      {page === 'appVersionManage' && <AppVersion />}
      {page === 'systemUpdate' && <SystemUpdate />}
      {page === 'clientReleaseManage' && <ClientRelease onNavigate={changePage} />}
      {page === 'adminLogManage' && <AdminLogs />}
      {page === 'adminUserManage' && <AdminUsers />}
      {page === 'adminRoleManage' && <Roles />}
      {page === 'serviceAgreement' && (
        <AgreementPage code="termsOfService" title="服务协议" description="配置后台登录服务协议内容与启用状态。" />
      )}
      {page === 'privacyAgreement' && (
        <AgreementPage code="privacyPolicy" title="隐私政策" description="配置后台登录隐私政策内容与启用状态。" />
      )}
      {page === 'accountManage' && <AccountManage />}
      {page === 'taxRateManage' && <TaxRates />}
      {page === 'shippingTypeManage' && <ShippingTypes />}
      {page === 'logisticsManage' && <ShippingSettingsPage />}
      {page === 'logisticsCompanyConfigManage' && <LogisticsSettings />}
      {page === 'payLogManage' && <PayLogs />}
      {page === 'refundLogManage' && <RefundLogs />}
      {page === 'userBalanceLogManage' && <BalanceLogs />}
      {page === 'staffFundWalletLogManage' && <StaffWalletLogs />}
      {page === 'buyerShowManage' && <BuyerShow />}
      {page === 'integralLogManage' && <IntegralLogs />}
      {page === 'refundApplyManage' && <RefundApplyList />}
      {page === 'userWithdrawApplyManage' && <WithdrawApplyList />}
      {page === 'userRechargeOrderManage' && <RechargeOrderList />}
      {page === 'userInvoiceManage' && <UserInvoiceList />}
      {page === 'orderInvoiceManage' && <OrderInvoiceList />}
      {page === 'depositRuleManage' && <DepositRules />}
      {page === 'depositPayeeAccountManage' && <DepositAccounts />}
      {page === 'depositAccountManage' && <DepositQuery />}
      {page === 'depositPaymentAuditManage' && <DepositAudit />}
      {page === 'depositLogManage' && <DepositLogs />}
      {page === 'categoryFeeRuleManage' && <CategoryFeeRules />}
      {page === 'merchantApplicationManage' && <MerchantApplications />}
      {page === 'subMerchantManage' && <SubMerchants />}
      {page === 'profitSharingSettlementManage' && <ProfitSharing />}
      {page === 'statementDownload' && <StatementDownload />}
      {page === 'pos0' && <PosWorkbench />}
      {page === 'posCashierDeviceList' && <CashierDevices />}
      {page === 'posReceiptPrint' && <ReceiptPrintersPage />}
      {page === 'posCashierList' && <PosCashierList />}
      {page === 'paymentSetting' && <PaymentSettings />}
      {page === 'withdrawalSettingManage' && <WithdrawalSettings />}
      {page === 'profitsharingSettingManage' && <ProfitSharingSettings />}
      {page === 'depositSettingManage' && <DepositSettings />}
      {page === 'priceDisplaySettings' && <PriceDisplaySettings />}
      {page === 'paidCancelRefundSettings' && <PaidCancelRefundSettings />}
      {page === 'giftOrderSettings' && <GiftOrderSettings />}
      {page === 'kitchenBoard' && <KitchenBoard />}
      {page === 'buyStyleSettings' && <BuyStyleSettings />}
      {page === 'systemMode' && <SystemModeSettings />}
      {page === 'federationSettings' && <FederationSettings />}
      {page === 'siteClone' && <SiteClone />}
      {page === 'takeoutManage' && <TakeoutManage />}
      {page === 'storeMapManage' && <StoreMapDistribution />}
      {page === 'platformAnnouncement' && <PlatformAnnouncements />}
      {page === 'orgOverview' && <OrgOverview />}
      {page === 'hubLinks' && <HubLinks />}
      {page === 'hubPairing' && <HubPairing />}
      {page === 'myLinks' && <MyLinks />}
      {page === 'hubCredentials' && <HubCredentials />}
      {page === 'hubAggregation' && <HubAggregation />}
      {page === 'hubMembers' && <HubMembers />}
      {page === 'hubOrders' && <HubOrders />}
      {page === 'hubSso' && <HubSso />}
      {page === 'hubConfig' && <HubConfig />}
      {page === 'feedKeyManage' && <FeedKeys />}
      {page === 'fontSettings' && <FontSettings />}
      {page === 'adSettingsManage' && <AdSettingsPage />}
      {page === 'adCampaignManage' && <AdCampaignManage />}
      {page === 'adPromotionManage' && <AdPromotionManage />}
      {page === 'shopSettingsManage' && <ShopSettings />}
      {page === 'storeSettingsManage' && <StoreSettings />}
      {page === 'storeTipManage' && <StoreTips />}
      {page === 'storeAreaManage' && <StoreAreas />}
      {page === 'storeListManage' && <StoreList />}
      {page === 'storeSelfPackListManage' && <SelfPackList />}
      {page === 'deliverymanList' && <DeliverymanList />}
      {page === 'servicemanList' && <ServicemanList />}
      {page === 'staffApplyManage' && <StaffApplyManage />}
      {page === 'deliveryOrderManage' && <DeliveryOrderManage />}
      {page === 'riderTrackManage' && <RiderTrackManage />}
      {(page === 'statementDetails' || page === 'vendorStatementDetails') && <StatementDetails source={page === 'vendorStatementDetails' ? 'vendor' : 'shop'} />}
      {page === 'accountPanelManage' && <AccountPanel />}
      {page === 'distributionCommissionManage' && <DistributionCommission />}
      {page === 'shopWithdrawApplySummary' && <ShopWithdrawApplySummary />}
      {page === 'staffFundWalletManage' && <StaffFundWallet />}
      {page === 'wechatMenu' && <WechatMenu />}
      {page === 'wechatKeywordReply' && <WechatKeywordReply />}
      {page === 'wechatAutoReply' && <WechatAutoReply />}
      {page === 'voiceBroadcastManage' && <VoiceBroadcast />}
      {page === 'promotion0' && <PromotionOverview />}
      {page === 'promotion2' && <PromotionStyle />}
      {page === 'rewardGiftManage' && <RewardGift />}
      {page === 'signInSettingManage' && <SignInSetting />}
      {page === 'eCardManage' && <ECard />}
      {page === 'giftCardManage' && <GiftCards />}
      {page === 'lotteryManage' && <Lottery />}
      {(page === 'decorate0' || page === 'decorate') && <ThemeStyle />}
      {(page === 'mobileDecorateManage' || page === 'decorate1' || page === 'mobileDecorate') && <MobileDecoratePages />}
      {(page === 'mobileSplashAdManage' || page === 'mobileSplashAdManage2') && <MobileSplashAd />}
      {page === 'mobileuserUserOverseasManage' && <MobileUserOverseas />}
      {page === 'mobileSearchStyleManage' && <MobileSearchStyle />}
      {page === 'mobileNavManage' && <MobileNav />}
      {page === 'classifyManage' && <ClassifyDecorate />}
      {page === 'mobileUserManage' && <MobileUserDecorate />}
      {page === 'mobileCatNavManage' && <MobileCatNav />}
      {(page === 'pcDecorateManage' || page === 'decorate2' || page === 'pcDecorate' || page === 'pcDecorateEdit') && <PcDecorateHome />}
      {page === 'pcNavigationManage' && <PcNavigation />}
      {page === 'pcCatFloorManage' && <PcCatFloor />}
      {(page === 'pcDecorateCustomManage' || page === 'pcCustomPage') && <PcCustomPage />}
      {page === 'pcProductStyleManage' && <PcProductStyle />}
      {page === 'pcSearchStyleManage' && <PcSearchStyle />}
      {page === 'pcDecorateOtherManage' && <PcDecorateOther />}
      {page === 'pcSearchDecorateManage' && <PcSearchDecorate />}
      {page === 'pcLoginDecorateManage' && <PcLoginDecorate />}
      {page === 'customerService0' && <KefuWorkbench />}
      {page === 'kefuHistoryList' && <KefuHistory />}
      {page === 'afterSalesServiceManage' && (
        <div style={{ padding: 40, textAlign: 'center' }}>
          <div style={{ fontSize: 20, fontWeight: 'bold', marginBottom: 12 }}>商品售后服务管理</div>
          <div style={{ color: '#888' }}>该模块能力已由统一商品服务管理 (productServicesManage) 全面覆盖。</div>
        </div>
      )}
      {page === 'shopWithdrawApplyManage' && <ShopWithdrawApplies />}
      {page === 'vendorWithdrawApplyManage' && <VendorWithdrawApplies />}
      {page === 'vendorStatementDownload' && <VendorStatementDownload />}
      {page === 'vendorWithdrawApplySummary' && <VendorWithdrawSummary />}
      {page === 'deliveryStaffWithdrawManage' && <DeliveryStaffWithdraw />}
      {page === 'serverStaffWithdrawManage' && <ServerStaffWithdraw />}
      {page === 'wholesalerManage' && <Wholesalers />}
      {page === 'wholesalerGradeManage' && <WholesalerGrades />}
      {page === 'wholesaleSettingManage' && <WholesaleSetting />}
      {page === 'wholesaleProductManage' && <WholesaleProducts />}
      {page === 'wholesaleSummaryManage' && <WholesaleSummary />}
      {page === 'storePickupSettingManage' && <StorePickupSetting />}
      {page === 'storeAppointmentTplManage' && <StoreAppointmentTemplates />}
      {page === 'storeEntryRuleSetting' && <StoreEntryRuleSetting />}
      {page === 'wechatLiveMiniAppSetting' && <WechatLiveMiniAppSetting />}
      {page === 'shippingTplManage' && <ShippingTemplates />}
      {page === 'messageTypeManage' && <MessageTypes />}
      {page === 'consumerRanking' && <ConsumerRanking />}
      {page === 'saleTargets' && <SaleTargets />}
      {page === 'statisticsGoods' && <StatisticsGoods />}
      {page === 'statisticsUser' && <StatisticsUser />}
      {page === 'newMembers' && <NewMembers />}
      {page === 'statisticsAccess' && <StatisticsAccess />}
      {page === 'statisticsOrder' && <StatisticsOrder />}
      {page === 'statisticsSale' && <StatisticsSale />}
      {page === 'adminMerchantSettingsManage' && <AdminMerchantSettings />}
      {page === 'deliveryFundSettingsManage' && <StaffFundSettings apiPath="deliveryFundManagement" />}
      {page === 'serverFundSettingsManage' && <StaffFundSettings apiPath="serverFundManagement" />}
      {page === 'deliverManage' && <DeliverOrders />}
      {page === 'exchangeOrderManage' && <DeliverOrders variant="exchange" />}
      {page === 'cycleBuyManage' && <DeliverOrders variant="cycle" />}
      {page === 'proxyShoppingManage' && <ProxyShopping />}
      {page === 'proxyHallManage' && <ProxyHall />}
      {page === 'aftersalesManage' && <AftersalesOrders />}
      {page === 'orderExportManage' && <OrderExport />}
      {page === 'couponManage' && <Coupons />}
      {page === 'rechargeSettingManage' && <RechargeSettings />}
      {page === 'rechargeGiftRecordManage' && <RechargeGiftRecords />}
      {page === 'marketingDistributionManage' && <MarketingDistribution />}
      {page === 'pointsExchangeManage' && <PointsExchanges />}
      {page === 'productGiftManage' && <ProductGifts />}
      {page === 'seckillManage' && <Seckills />}
      {page === 'auctionItemManage' && <AuctionItems />}
      {page === 'auctionSessionManage' && <AuctionSessions />}
      {page === 'auctionDepositManage' && <AuctionDeposits />}
      {page === 'auctionBreachManage' && <AuctionBreaches />}
      {page === 'auctionSettings' && <AuctionSettings />}
      {page === 'limitdiscountManage' && <TimeDiscounts />}
      {page === 'fullReductionManage' && <ProductPromotions />}
      {page === 'grouponPromotionManage' && <Groupons />}
      {page === 'inventoryQueryManage' && <InventoryQuery />}
      {page === 'productInventoryLogManage' && <ProductInventoryLogs />}
      {page === 'inventoryAdjustmentManage' && <InventoryAdjustments />}
      {page === 'inventoryGoodsArePutIntoStorageManage' && <StockRecords inOrOut={1} />}
      {page === 'inventoryGoodsArePutOutOfStorageManage' && <StockRecords inOrOut={2} />}
      {page === 'storeInventoryTransferManage' && <InventoryTransfers />}
      {page === 'enquiryManage' && <Enquiries />}
      {page === 'stockWarningManage' && <StockWarnings />}
      {page === 'productBatchExportManage' && <ProductBatch mode="export" />}
      {page === 'productBatchAddManage' && <ProductBatch mode="add" />}
      {page === 'productBatchEditManage' && <ProductBatch mode="edit" />}
      {page === 'storeInventoryCalendarManage' && <InventoryCalendars />}
      {page === 'memberTagManage' && <MemberTags />}
      {page === 'feedbackManage' && <Feedbacks />}
      {page === 'messageLogManage' && <MessageLogs />}
      {page === 'levelManageManage' && <MemberLevels />}
      {page === 'shopMemberSettingManage' && <ShopMemberSettings />}
      {page === 'shopMemberCustomerManage' && <ShopCustomers />}
      {page === 'userAuthenticationManage' && <UserAuthenticationSettings />}
      {page === 'userLevelProManage' && <MemberLevelPros />}
      {page === 'articleCategoryManage' && <ArticleCategories />}
      {page === 'kefuManageList' && <KefuList />}
      {page === 'kefuAccessSetting' && <KefuAccessSettings />}
      {page === 'kefuQuickReply' && <KefuQuickReplies />}
      {page === 'kefuAccountInfo' && <KefuAccountInfo />}
      {page === 'wechatLive' && <WechatLives />}
      {page === 'wechatLiveProduct' && <WechatLiveGoods />}
      {(page === 'videoNumberManage' || page === 'videoNumber') && <VideoNumbers />}
      {page === 'baseReceiptManage' && <ReceiptPrintersPage />}
      {page === 'salesman0' && <SalesmanOverview />}
      {page === 'analysisTableManage' && <SalesmanAnalysisTable />}
      {page === 'rankingTableManage' && <SalesmanRankingTable />}
      {page === 'promoteModeManage' && <SalesmanPromoteMode />}
      {page === 'salesmanProductManage' && <SalesmanProducts />}
      {page === 'promoterManage' && <SalesmanPromoters />}
      {page === 'subgroupManage' && <SalesmanSubgroup />}
      {page === 'salesmanDetailedManage' && <SalesmanDetailed />}
      {page === 'customerTransactionsManage' && <CustomerTransactions />}
      {page === 'performanceSettlementSettingManage' && <SalesmanSettlementSetting />}
      {page === 'performanceSettlementManage' && <SalesmanPerformanceSettlement />}
      {page === 'salesmanNoticeManage' && <SalesmanNotice />}
      {page === 'materialCategoryManage' && <SalesmanMaterialCategory />}
      {page === 'materialManage' && <SalesmanMaterial />}
      {page === 'premiumMemberManage' && <PremiumMembers />}
      {page === 'premiumMemberRightManage' && <PremiumMemberRights />}
      {page === 'intraCityDeliveryManage' && <IntraCityDelivery />}
      {page === 'homeServiceManage' && <HomeService />}
      {page === 'languagesListManage' && <LanguagesList />}
      {page === 'currencyManagementManage' && <CurrencyManagement />}
      {page === 'translationContentManage' && <TranslationContent />}
      {page === 'verbalAssociationManage' && <VerbalAssociation />}
      {page === 'areaCodeManage' && <AreaCode />}
      {page === 'aiImageSettingManage' && <AiImageSetting />}
      {page === 'statementDetails' && <StatementDetails source="shop" />}
      {page === 'vendorStatementDetails' && <StatementDetails source="vendor" />}
      {page === 'vendorAccountManage' && <VendorAccountManage />}
      {page === 'vendorFundsManage' && <VendorFundsManage />}
      {page === 'vendorFinancialLogManage' && <VendorFinancialLogManage />}
      {page === 'shopAccountManage' && <ShopAccountOverview />}
      {page === 'shopFundsManage' && <ShopFundsManage />}
      {/* financialLogManage 分支已移至下方「角色自助页面集」，按 adminType 分流 */}
      {(page === 'licensed' || page === 'copyright') && <LicensedPage />}
      {page === 'EmployeeManagement' && <ShopStaffManage />}
      {page === 'vendorUserManage' && <VendorStaffManage />}
      {page === 'bindShopManager' && <VendorBindShopManage />}
      {page === 'ShopTeamManage' && <ShopTeamSettings />}
      {page === 'ShopInfoManage' && <ShopInfoDashboard />}
      {/* ===== 角色自助页面集（商户端/供应商端，报告 §5 孤儿域）=====
       * dashboardManage / balanceManage / withdrawManage / accountListManage
       * 两端共用 key，按 adminType 分流到对应端点组件。 */}
      {page === 'dashboardManage' && (localStorage.getItem('adminType') === 'vendor' ? <VendorDashboardManage /> : <ShopDashboardManage />)}
      {page === 'balanceManage' && (localStorage.getItem('adminType') === 'vendor' ? <VendorBalanceManage /> : <ShopBalanceManage />)}
      {page === 'withdrawManage' && (localStorage.getItem('adminType') === 'vendor' ? <VendorWithdrawManage /> : <ShopWithdrawManage />)}
      {page === 'accountListManage' && (localStorage.getItem('adminType') === 'vendor' ? <VendorAccountListManage /> : <ShopAccountListManage />)}
      {page === 'shopDepositManage' && <ShopDepositManage />}
      {page === 'vendorDepositManage' && <VendorDepositManage />}
      {page === 'financialLogManage' && (localStorage.getItem('adminType') === 'vendor' ? <VendorFinancialLogManage /> : <ShopFinancialLogSelfManage />)}
      {page === 'merchantApplyManage' && <MerchantApplies />}
      {page === 'shopProductCategoryManage' && <ShopProductCategoryManage />}
      {page === 'vendorInfoManage' && <VendorInfoManage />}
      {page === 'vendorProductStockManage' && <VendorProductStockManage />}
      {page === 'vendorStockLogManage' && <VendorStockLogManage />}
      {page === 'printDesignerManage' && <PrintDesigner />}
      {page === 'EmployeeLogManagement' && <AdminLogs />}
      {page === 'offlinePaymentClaimManage' && <OfflinePaymentClaimManage />}
      {page === 'imageSearchManage' && <ImageSearchManage />}
      {page === 'shopGroupManage' && <ShopGroupManage />}
      {page === 'shopTipManage' && <ShopTipManage />}
      {page === 'purchaseOrderManage' && <PurchaseOrders />}
      {page === 'purchaseReceiptManage' && <PurchaseInbound />}
      {page === 'purchaseReturnManage' && <PurchaseReturns />}
      {page === 'purchaseReconciliationManage' && <PurchaseReconciliation />}
      {page === 'selfVendorManage' && <SupplierList />}
      {page === 'selfVendorCategoryManage' && <SupplierCategories />}
      {page === 'selfVendorProductManage' && <SupplierProducts />}
      {page === 'purchaseSettingManage' && <PurchaseSetting />}
      {page === 'authorityManage' && <Authorities />}
      {page === 'materialCenter' && <MaterialCenter />}
      {/* baseNoticeManage 旧版是四 Tab 页（短信/邮件/钉钉/企业微信各走独立接口），不走通用配置页 */}
      {page === 'baseNoticeManage' && <NoticeSettings />}
      {page === 'termOrderManage' && <CreditOrders />}
      {page === 'termPayManage' && <CreditSettlements />}
      {page === 'termMemberManage' && <CreditUsers />}
      {page === 'userCertificationManage' && <UserCertifications />}
      {page === 'shopManage' && <ShopList />}
      {page === 'shopLabelManage' && <ShopLabelManage />}
      {page === 'salesmanShopManage' && <SalesmanShopManage />}
      {page === 'shopMiniProgramManage' && <ShopMiniProgramManage />}
      {page === 'applyManage' && <MerchantApplies />}
      {page === 'adminMerchantManage' && <Merchants />}
      {page === 'investmentManage' && <InvestmentContent />}
      {page === 'vendorListManage' && <VendorList />}
      {page === 'vendorProductManage' && <VendorProducts />}
      {page === 'vendorOrderManage' && <VendorOrders />}
      {/* storeSettingsManage 有专用页面 StoreSettings（自带完整 sections/renderers），
          避免与通用 AutoConfigPage 同页重复渲染两份表单。
          kefuSetting 同理走专用 KefuSettings（对齐旧版 ServiceSettings.vue 的
          按钮式客服选择 + 按类型条件渲染 + 敏感值编辑弹窗）。 */}
      {configPages[page] && page !== 'storeSettingsManage' && page !== 'kefuSetting' && (
        <AutoConfigPage
          {...configPages[page]}
          onSaved={page === 'globalSetting' || page === 'seoSettings' ? applyGlobalSettings : undefined}
        />
      )}
      {page === 'kefuSetting' && <KefuSettings />}
      {page === 'afterSaleService' && (
        <AgreementPage code="afterSalesService" title="售后服务" description="配置售后服务协议内容与启用状态。" />
      )}
      {page === 'orders' && <Orders onNavigate={changePage} />}
      {page === 'members' && <Members />}
      {page === 'customerBlacklistManage' && <CustomerBlacklist />}
      {page === 'content' && <Content />}
      {!knownPages.has(page) && <Placeholder page={page} />}
      </Suspense>
    </AdminLayout>
  );
}
