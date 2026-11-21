//! 站点搬家装修/素材域仓储（site_clone::decorate_data，P2）：
//! - 导出侧：装修页/离散组件/导航/楼层/移动分类导航/素材库的全量只读查询
//!   （装修页与素材按源店铺圈定，素材含平台 shop_id=0；导航三表为平台级）；
//! - 导入侧：装修页新建与父子/绑定关系回填、离散组件与导航三表的落库、
//!   素材分组对齐与本地素材行落库、转存映射（`site_clone_asset_map`）、
//!   已导入商品与外链登记素材的 URL 重写读写、撤销导入的删除。
//!
//! SQL 一律运行时 `sqlx::query` / `query_as`，`$n` 占位（与同模块 clone.rs 同法）。

use sqlx::{FromRow, PgPool};

use nextrade_core::models::site_clone::clone::{
    CloneAssetGroup, CloneAssetPic, CloneAssetVideo, CloneAssetVideoGroup, CloneDecoratePage,
    CloneDiscrete, CloneMobileCatNav, ClonePcCatFloor, ClonePcNavigation, SiteCloneAssetMap,
};
use nextrade_core::{Error, Result};

/// 装修/素材域仓储。
#[derive(Clone)]
pub struct SiteCloneDecorateRepository {
    pool: PgPool,
}

impl SiteCloneDecorateRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ———————————————————— 导出：计数 ————————————————————

    pub async fn count_decorate_pages(&self, shop_id: i32) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM decorate WHERE shop_id = $1")
                .bind(shop_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    pub async fn count_discrete(&self, shop_id: i32) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM decorate_discrete WHERE shop_id = $1")
                .bind(shop_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    /// 导航三表合计（平台级结构，无店铺维）。
    pub async fn count_nav_total(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT (SELECT COUNT(*) FROM pc_navigation)
                  + (SELECT COUNT(*) FROM pc_cat_floor)
                  + (SELECT COUNT(*) FROM mobile_cat_nav)",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    pub async fn count_asset_pics(&self, shop_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM gallery_pic WHERE shop_id IN (0, $1)",
        )
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    pub async fn count_asset_videos(&self, shop_id: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM gallery_video_info WHERE shop_id IN (0, $1)",
        )
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0.max(0))
    }

    // ———————————————————— 导出：装修 ————————————————————

    pub async fn list_decorate_pages(&self, shop_id: i32) -> Result<Vec<CloneDecoratePage>> {
        sqlx::query_as::<_, CloneDecoratePage>(
            "SELECT decorate_id, decorate_title, data, draft_data, decorate_type, page_type,
                    is_home, status, locale_id, parent_id, sort_order, binded_h5_id, share_info
             FROM decorate WHERE shop_id = $1 ORDER BY decorate_id ASC",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_discrete(&self, shop_id: i32) -> Result<Vec<CloneDiscrete>> {
        sqlx::query_as::<_, CloneDiscrete>(
            "SELECT id, decorate_sn, decorate_name, data
             FROM decorate_discrete WHERE shop_id = $1 ORDER BY id ASC",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_navigations(&self) -> Result<Vec<ClonePcNavigation>> {
        sqlx::query_as::<_, ClonePcNavigation>(
            "SELECT id, title, is_show, sort_order, is_blank, link, \"type\", parent_id,
                    icon, entry_type, qr_type, qr_image
             FROM pc_navigation ORDER BY \"type\" ASC, sort_order ASC, id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_cat_floors(&self) -> Result<Vec<ClonePcCatFloor>> {
        sqlx::query_as::<_, ClonePcCatFloor>(
            "SELECT cat_floor_id, cat_floor_name, category_ids, category_names, floor_ico,
                    hot_cat, is_show, sort_order, floor_ico_font, brand_ids
             FROM pc_cat_floor ORDER BY sort_order ASC, cat_floor_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_mobile_cat_navs(&self) -> Result<Vec<CloneMobileCatNav>> {
        sqlx::query_as::<_, CloneMobileCatNav>(
            "SELECT mobile_cat_nav_id, category_id, cat_color, child_cat_ids, brand_ids,
                    is_show, img_url, sort_order, cat_name_alias
             FROM mobile_cat_nav ORDER BY sort_order ASC, mobile_cat_nav_id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    // ———————————————————— 导出：素材 ————————————————————

    pub async fn list_asset_groups(&self, shop_id: i32) -> Result<Vec<CloneAssetGroup>> {
        sqlx::query_as::<_, CloneAssetGroup>(
            "SELECT gallery_id, parent_id, gallery_name, gallery_sort, gallery_thumb, shop_id
             FROM gallery WHERE shop_id IN (0, $1) ORDER BY parent_id ASC, gallery_sort ASC, gallery_id ASC",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_asset_pics(&self, shop_id: i32) -> Result<Vec<CloneAssetPic>> {
        sqlx::query_as::<_, CloneAssetPic>(
            "SELECT pic_id, gallery_id, shop_id, pic_url, pic_name, pic_thumb,
                    pic_width, pic_height, pic_size, file_type, pic_sort
             FROM gallery_pic WHERE shop_id IN (0, $1) ORDER BY gallery_id ASC, pic_sort ASC, pic_id ASC",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_asset_video_groups(
        &self,
        shop_id: i32,
    ) -> Result<Vec<CloneAssetVideoGroup>> {
        sqlx::query_as::<_, CloneAssetVideoGroup>(
            "SELECT id, parent_id, name, sort, shop_id
             FROM gallery_video WHERE shop_id IN (0, $1) ORDER BY parent_id ASC, sort ASC, id ASC",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn list_asset_videos(&self, shop_id: i32) -> Result<Vec<CloneAssetVideo>> {
        sqlx::query_as::<_, CloneAssetVideo>(
            "SELECT id, gallery_id, shop_id, video_url, video_name, video_cover,
                    format, video_first_frame, duration, size, description
             FROM gallery_video_info WHERE shop_id IN (0, $1) ORDER BY gallery_id ASC, id ASC",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    // ———————————————————— 导入：装修页 ————————————————————

    /// 新建装修页（导入恒未发布 status=0、非首页 is_home=0；父子/绑定关系
    /// 先占位 0，全部页落库后由 `update_decorate_links` 按映射回填）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_decorate_page(
        &self,
        title: &str,
        data: Option<&str>,
        draft_data: Option<&str>,
        decorate_type: i32,
        page_type: i32,
        shop_id: i32,
        locale_id: i32,
        sort_order: Option<i32>,
        share_info: Option<&str>,
        now: i32,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO decorate (decorate_title, data, draft_data, decorate_type, page_type,
                                   is_home, shop_id, status, update_time, locale_id, parent_id,
                                   sort_order, binded_h5_id, share_info)
             VALUES ($1, $2, $3, $4, $5, 0, $6, 0, $7, $8, 0, $9, 0, $10) RETURNING decorate_id",
        )
        .bind(title)
        .bind(data)
        .bind(draft_data)
        .bind(decorate_type)
        .bind(page_type)
        .bind(shop_id)
        .bind(now)
        .bind(locale_id)
        .bind(sort_order)
        .bind(share_info)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 回填装修页的父页与绑定 H5 页（导入第二遍）。
    pub async fn update_decorate_links(
        &self,
        decorate_id: i32,
        parent_id: i32,
        binded_h5_id: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE decorate SET parent_id = $2, binded_h5_id = $3 WHERE decorate_id = $1",
        )
        .bind(decorate_id)
        .bind(parent_id)
        .bind(binded_h5_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn find_discrete(
        &self,
        shop_id: i32,
        decorate_sn: &str,
    ) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT id FROM decorate_discrete WHERE shop_id = $1 AND decorate_sn = $2 ORDER BY id ASC LIMIT 1",
        )
        .bind(shop_id)
        .bind(decorate_sn)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(id,)| id))
    }

    pub async fn insert_discrete(
        &self,
        shop_id: i32,
        decorate_sn: &str,
        decorate_name: Option<&str>,
        data: &str,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO decorate_discrete (decorate_sn, decorate_name, data, shop_id)
             VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(decorate_sn)
        .bind(decorate_name)
        .bind(data)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    // ———————————————————— 导入：导航三表 ————————————————————

    pub async fn insert_navigation(
        &self,
        n: &ClonePcNavigation,
        parent_id: i32,
        link: &str,
        icon: &str,
        qr_image: &str,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO pc_navigation (title, is_show, sort_order, is_blank, link, \"type\",
                                        parent_id, icon, entry_type, qr_type, qr_image)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id",
        )
        .bind(&n.title)
        .bind(n.is_show)
        .bind(n.sort_order)
        .bind(n.is_blank)
        .bind(link)
        .bind(n.r#type)
        .bind(parent_id)
        .bind(icon)
        .bind(&n.entry_type)
        .bind(&n.qr_type)
        .bind(qr_image)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_cat_floor(
        &self,
        f: &ClonePcCatFloor,
        category_ids: &str,
        category_names: &str,
        floor_ico: &str,
        brand_ids: &str,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO pc_cat_floor (cat_floor_name, category_ids, category_names, floor_ico,
                                       hot_cat, is_show, sort_order, floor_ico_font, brand_ids)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING cat_floor_id",
        )
        .bind(&f.cat_floor_name)
        .bind(category_ids)
        .bind(category_names)
        .bind(floor_ico)
        .bind(&f.hot_cat)
        .bind(f.is_show)
        .bind(f.sort_order)
        .bind(&f.floor_ico_font)
        .bind(brand_ids)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_mobile_cat_nav(
        &self,
        n: &CloneMobileCatNav,
        category_id: i32,
        child_cat_ids: &str,
        brand_ids: &str,
        img_url: &str,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO mobile_cat_nav (category_id, cat_color, child_cat_ids, brand_ids,
                                          is_show, img_url, sort_order, cat_name_alias)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING mobile_cat_nav_id",
        )
        .bind(category_id)
        .bind(&n.cat_color)
        .bind(child_cat_ids)
        .bind(brand_ids)
        .bind(n.is_show)
        .bind(img_url)
        .bind(n.sort_order)
        .bind(&n.cat_name_alias)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    // ———————————————————— 导入：素材库 ————————————————————

    pub async fn find_gallery_group(
        &self,
        shop_id: i32,
        name: &str,
        parent_id: i32,
    ) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT gallery_id FROM gallery WHERE shop_id = $1 AND gallery_name = $2 AND parent_id = $3 ORDER BY gallery_id ASC LIMIT 1",
        )
        .bind(shop_id)
        .bind(name)
        .bind(parent_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(id,)| id))
    }

    pub async fn insert_gallery_group(
        &self,
        shop_id: i32,
        parent_id: i32,
        name: &str,
        sort: i32,
        thumb: &str,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO gallery (parent_id, gallery_admin_id, gallery_name, gallery_sort, gallery_thumb, shop_id, vendor_id, is_shared)
             VALUES ($1, 0, $2, $3, $4, $5, 0, 0) RETURNING gallery_id",
        )
        .bind(parent_id)
        .bind(name)
        .bind(sort)
        .bind(thumb)
        .bind(shop_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 转存后的图片落素材库（source='local'，文件已在本站存储）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_gallery_pic(
        &self,
        shop_id: i32,
        gallery_id: i32,
        pic_url: &str,
        pic_name: &str,
        pic_thumb: &str,
        width: Option<i32>,
        height: Option<i32>,
        size: Option<i64>,
        file_type: Option<&str>,
        pic_sort: Option<i32>,
        add_time: &str,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO gallery_pic (shop_id, gallery_id, pic_ower_id, pic_url, pic_name, pic_thumb,
                                      add_time, vendor_id, hidden, pic_width, pic_height, pic_size,
                                      file_type, pic_sort, source)
             VALUES ($1, $2, 0, $3, $4, $5, $6, 0, 0, $7, $8, $9, $10, $11, 'local') RETURNING pic_id",
        )
        .bind(shop_id)
        .bind(gallery_id)
        .bind(pic_url)
        .bind(pic_name)
        .bind(pic_thumb)
        .bind(add_time)
        .bind(width)
        .bind(height)
        .bind(size)
        .bind(file_type)
        .bind(pic_sort)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    pub async fn find_video_group(
        &self,
        shop_id: i32,
        name: &str,
        parent_id: i32,
    ) -> Result<Option<i32>> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT id FROM gallery_video WHERE shop_id = $1 AND name = $2 AND parent_id = $3 ORDER BY id ASC LIMIT 1",
        )
        .bind(shop_id)
        .bind(name)
        .bind(parent_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|(id,)| id))
    }

    pub async fn insert_video_group(
        &self,
        shop_id: i32,
        parent_id: i32,
        name: &str,
        sort: i32,
        now: i32,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO gallery_video (shop_id, parent_id, name, sort, add_time, add_user_id, vendor_id, is_shared)
             VALUES ($1, $2, $3, $4, $5, 0, 0, 0) RETURNING id",
        )
        .bind(shop_id)
        .bind(parent_id)
        .bind(name)
        .bind(sort)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    /// 转存后的视频落素材库（source='local'）。
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_video_info(
        &self,
        shop_id: i32,
        gallery_id: i32,
        video_url: &str,
        video_name: &str,
        video_cover: Option<&str>,
        format: Option<&str>,
        first_frame: Option<&str>,
        duration: Option<&str>,
        size: Option<&str>,
        description: Option<&str>,
        now: i32,
    ) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "INSERT INTO gallery_video_info (shop_id, gallery_id, video_url, video_name, video_cover,
                                             add_time, add_user_id, format, video_first_frame, duration,
                                             size, vendor_id, source, description)
             VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8, $9, $10, 0, 'local', $11) RETURNING id",
        )
        .bind(shop_id)
        .bind(gallery_id)
        .bind(video_url)
        .bind(video_name)
        .bind(video_cover)
        .bind(now)
        .bind(format)
        .bind(first_frame)
        .bind(duration)
        .bind(size)
        .bind(description)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.0)
    }

    // ———————————————————— 转存映射 ————————————————————

    pub async fn find_asset_map(
        &self,
        fingerprint: &str,
        source_url: &str,
    ) -> Result<Option<SiteCloneAssetMap>> {
        sqlx::query_as::<_, SiteCloneAssetMap>(
            "SELECT * FROM site_clone_asset_map WHERE fingerprint = $1 AND source_url = $2",
        )
        .bind(fingerprint)
        .bind(source_url)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_asset_map(
        &self,
        fingerprint: &str,
        source_url: &str,
        local_url: &str,
        media_kind: &str,
        target_pic_id: i32,
        job_id: i32,
        now: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO site_clone_asset_map (fingerprint, source_url, local_url, media_kind, target_pic_id, job_id, add_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (fingerprint, source_url) DO NOTHING",
        )
        .bind(fingerprint)
        .bind(source_url)
        .bind(local_url)
        .bind(media_kind)
        .bind(target_pic_id)
        .bind(job_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    /// 素材库落库后回填 target_pic_id（供撤销导入时定位删除）。
    pub async fn set_asset_map_pic(
        &self,
        fingerprint: &str,
        source_url: &str,
        target_pic_id: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE site_clone_asset_map SET target_pic_id = $3 WHERE fingerprint = $1 AND source_url = $2",
        )
        .bind(fingerprint)
        .bind(source_url)
        .bind(target_pic_id)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn list_asset_maps_by_job(&self, job_id: i32) -> Result<Vec<SiteCloneAssetMap>> {
        sqlx::query_as::<_, SiteCloneAssetMap>(
            "SELECT * FROM site_clone_asset_map WHERE job_id = $1 ORDER BY map_id ASC",
        )
        .bind(job_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn delete_asset_maps_by_job(&self, job_id: i32) -> Result<()> {
        sqlx::query("DELETE FROM site_clone_asset_map WHERE job_id = $1")
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(())
    }

    // ———————————————————— URL 重写：外链登记素材与已导入商品 ————————————————————

    /// P1 登记的 external 素材行（回溯转存的输入）。
    pub async fn list_external_pics(
        &self,
        shop_id: i32,
    ) -> Result<Vec<(i32, String, String)>> {
        let rows: Vec<(i32, String, String)> = sqlx::query_as(
            "SELECT pic_id, pic_url, pic_thumb FROM gallery_pic
             WHERE shop_id = $1 AND source = 'external' ORDER BY pic_id ASC",
        )
        .bind(shop_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows)
    }

    /// 外链素材转存完成：URL 换本站地址、来源改 local。
    pub async fn update_pic_local(
        &self,
        pic_id: i32,
        pic_url: &str,
        pic_thumb: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE gallery_pic SET pic_url = $2, pic_thumb = $3, source = 'local' WHERE pic_id = $1",
        )
        .bind(pic_id)
        .bind(pic_url)
        .bind(pic_thumb)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn load_product_media(
        &self,
        product_id: i32,
    ) -> Result<Option<ProductMediaRow>> {
        sqlx::query_as::<_, ProductMediaRow>(
            "SELECT pic_url, pic_thumb, pic_original, pic_large, product_desc
             FROM product WHERE product_id = $1",
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn update_product_media(
        &self,
        product_id: i32,
        m: &ProductMediaRow,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE product SET pic_url = $2, pic_thumb = $3, pic_original = $4, pic_large = $5, product_desc = $6
             WHERE product_id = $1",
        )
        .bind(product_id)
        .bind(&m.pic_url)
        .bind(&m.pic_thumb)
        .bind(&m.pic_original)
        .bind(&m.pic_large)
        .bind(&m.product_desc)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn list_product_gallery_media(
        &self,
        product_id: i32,
    ) -> Result<Vec<ProductGalleryMediaRow>> {
        sqlx::query_as::<_, ProductGalleryMediaRow>(
            "SELECT pic_id, pic_url, pic_thumb, pic_original, pic_large
             FROM product_gallery WHERE product_id = $1 ORDER BY pic_id ASC",
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn update_product_gallery_media(
        &self,
        row: &ProductGalleryMediaRow,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE product_gallery SET pic_url = $2, pic_thumb = $3, pic_original = $4, pic_large = $5
             WHERE pic_id = $1",
        )
        .bind(row.pic_id)
        .bind(&row.pic_url)
        .bind(&row.pic_thumb)
        .bind(&row.pic_original)
        .bind(&row.pic_large)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn list_product_attr_pics(
        &self,
        product_id: i32,
    ) -> Result<Vec<ProductAttrPicRow>> {
        sqlx::query_as::<_, ProductAttrPicRow>(
            "SELECT attributes_id, attr_pic, attr_pic_thumb
             FROM product_attributes WHERE product_id = $1 ORDER BY attributes_id ASC",
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn update_product_attr_pics(&self, row: &ProductAttrPicRow) -> Result<()> {
        sqlx::query(
            "UPDATE product_attributes SET attr_pic = $2, attr_pic_thumb = $3 WHERE attributes_id = $1",
        )
        .bind(row.attributes_id)
        .bind(&row.attr_pic)
        .bind(&row.attr_pic_thumb)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    pub async fn list_product_video_media(
        &self,
        product_id: i32,
    ) -> Result<Vec<ProductVideoMediaRow>> {
        sqlx::query_as::<_, ProductVideoMediaRow>(
            "SELECT id, video_url, video_cover FROM product_video WHERE product_id = $1 ORDER BY id ASC",
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)
    }

    pub async fn update_product_video_media(
        &self,
        row: &ProductVideoMediaRow,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE product_video SET video_url = $2, video_cover = $3 WHERE id = $1",
        )
        .bind(row.id)
        .bind(&row.video_url)
        .bind(&row.video_cover)
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    // ———————————————————— 撤销导入：删除本任务新建对象 ————————————————————

    pub async fn delete_decorate_pages(&self, ids: &[i32]) -> Result<u64> {
        delete_in(&self.pool, "DELETE FROM decorate WHERE decorate_id IN ({})", ids).await
    }

    pub async fn delete_discrete(&self, ids: &[i32]) -> Result<u64> {
        delete_in(&self.pool, "DELETE FROM decorate_discrete WHERE id IN ({})", ids).await
    }

    pub async fn delete_navigations(&self, ids: &[i32]) -> Result<u64> {
        delete_in(&self.pool, "DELETE FROM pc_navigation WHERE id IN ({})", ids).await
    }

    pub async fn delete_cat_floors(&self, ids: &[i32]) -> Result<u64> {
        delete_in(&self.pool, "DELETE FROM pc_cat_floor WHERE cat_floor_id IN ({})", ids).await
    }

    pub async fn delete_mobile_cat_navs(&self, ids: &[i32]) -> Result<u64> {
        delete_in(
            &self.pool,
            "DELETE FROM mobile_cat_nav WHERE mobile_cat_nav_id IN ({})",
            ids,
        )
        .await
    }

    pub async fn delete_gallery_pics(&self, ids: &[i32]) -> Result<u64> {
        delete_in(&self.pool, "DELETE FROM gallery_pic WHERE pic_id IN ({})", ids).await
    }
}

/// 商品主表媒体字段行（URL 重写用）。
#[derive(Debug, Clone, Default, FromRow)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductMediaRow {
    pub pic_url: String,
    pub pic_thumb: String,
    pub pic_original: String,
    pub pic_large: String,
    pub product_desc: Option<String>,
}

/// 商品图库媒体字段行。
#[derive(Debug, Clone, Default, FromRow)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductGalleryMediaRow {
    pub pic_id: i32,
    pub pic_url: String,
    pub pic_thumb: String,
    pub pic_original: String,
    pub pic_large: String,
}

/// 商品属性图片字段行。
#[derive(Debug, Clone, Default, FromRow)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductAttrPicRow {
    pub attributes_id: i32,
    pub attr_pic: Option<String>,
    pub attr_pic_thumb: Option<String>,
}

/// 商品视频媒体字段行。
#[derive(Debug, Clone, Default, FromRow)]
#[sqlx(rename_all = "snake_case")]
pub struct ProductVideoMediaRow {
    pub id: i32,
    pub video_url: String,
    pub video_cover: String,
}

/// 按 id 集合删除（撤销导入专用；占位符个数由 ids 长度生成，无外部输入拼串）。
async fn delete_in(pool: &PgPool, template: &str, ids: &[i32]) -> Result<u64> {
    if ids.is_empty() {
        return Ok(0);
    }
    let placeholders = (1..=ids.len())
        .map(|i| format!("${i}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = template.replace("{}", &placeholders);
    let mut q = sqlx::query(&sql);
    for id in ids {
        q = q.bind(id);
    }
    let r = q.execute(pool).await.map_err(map_err)?;
    Ok(r.rows_affected())
}

fn map_err(e: sqlx::Error) -> Error {
    Error::Database(e.to_string())
}
