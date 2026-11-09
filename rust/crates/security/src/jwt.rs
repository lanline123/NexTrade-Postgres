//! JWT 令牌服务（HS256），替代原 Java java-jwt / jjwt。

use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};
use nextrade_common::config::JwtConfig;
use nextrade_core::{Error, Result};

/// 令牌类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    Access,
    Refresh,
}

/// 令牌声明（Claims）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// 主体：用户/管理员ID
    pub sub: String,
    /// 登录名
    pub usr: String,
    /// 角色ID
    pub role: Option<i32>,
    /// 管理员类型 admin/shop/vendor/suppliers
    pub typ: Option<String>,
    /// 店铺ID（多租户数据隔离：0 为平台，>0 为指定商户店铺）
    #[serde(default)]
    pub shop_id: Option<i32>,
    /// 供应商/供货商 ID（`admin_type=vendor/suppliers` 时有值）
    #[serde(default)]
    pub vendor_id: Option<i32>,
    /// 令牌类型
    pub ttype: TokenType,
    /// 签发时间（秒）
    pub iat: i64,
    /// 过期时间（秒）
    pub exp: i64,
    /// 签发者
    pub iss: String,
}

impl Claims {
    /// 访问令牌声明。
    pub fn access(
        sub: String,
        usr: String,
        role: Option<i32>,
        typ: Option<String>,
        cfg: &JwtConfig,
    ) -> Self {
        Self::new(sub, usr, role, typ, None, None, TokenType::Access, cfg.access_expire_secs, cfg)
    }

    /// 访问令牌声明（含店铺 ID）。
    pub fn access_with_shop(
        sub: String,
        usr: String,
        role: Option<i32>,
        typ: Option<String>,
        shop_id: Option<i32>,
        cfg: &JwtConfig,
    ) -> Self {
        Self::new(sub, usr, role, typ, shop_id, None, TokenType::Access, cfg.access_expire_secs, cfg)
    }

    /// 访问令牌声明（含店铺/供应商租户 ID）。
    pub fn access_with_tenant(
        sub: String,
        usr: String,
        role: Option<i32>,
        typ: Option<String>,
        shop_id: Option<i32>,
        vendor_id: Option<i32>,
        cfg: &JwtConfig,
    ) -> Self {
        Self::new(sub, usr, role, typ, shop_id, vendor_id, TokenType::Access, cfg.access_expire_secs, cfg)
    }

    /// 刷新令牌声明。
    pub fn refresh(
        sub: String,
        usr: String,
        role: Option<i32>,
        typ: Option<String>,
        cfg: &JwtConfig,
    ) -> Self {
        Self::new(sub, usr, role, typ, None, None, TokenType::Refresh, cfg.refresh_expire_secs, cfg)
    }

    /// 刷新令牌声明（含店铺 ID）。
    pub fn refresh_with_shop(
        sub: String,
        usr: String,
        role: Option<i32>,
        typ: Option<String>,
        shop_id: Option<i32>,
        cfg: &JwtConfig,
    ) -> Self {
        Self::new(sub, usr, role, typ, shop_id, None, TokenType::Refresh, cfg.refresh_expire_secs, cfg)
    }

    /// 刷新令牌声明（含店铺/供应商租户 ID）。
    pub fn refresh_with_tenant(
        sub: String,
        usr: String,
        role: Option<i32>,
        typ: Option<String>,
        shop_id: Option<i32>,
        vendor_id: Option<i32>,
        cfg: &JwtConfig,
    ) -> Self {
        Self::new(sub, usr, role, typ, shop_id, vendor_id, TokenType::Refresh, cfg.refresh_expire_secs, cfg)
    }

    fn new(
        sub: String,
        usr: String,
        role: Option<i32>,
        typ: Option<String>,
        shop_id: Option<i32>,
        vendor_id: Option<i32>,
        ttype: TokenType,
        expire_secs: i64,
        cfg: &JwtConfig,
    ) -> Self {
        let now = Utc::now();
        Self {
            sub,
            usr,
            role,
            typ,
            shop_id,
            vendor_id,
            ttype,
            iat: now.timestamp(),
            exp: (now + Duration::seconds(expire_secs)).timestamp(),
            iss: cfg.issuer.clone(),
        }
    }
}

/// JWT 签发/校验服务。
#[derive(Clone)]
pub struct JwtService {
    encoding: EncodingKey,
    decoding: DecodingKey,
    cfg: JwtConfig,
}

impl JwtService {
    pub fn new(cfg: JwtConfig) -> Self {
        Self {
            encoding: EncodingKey::from_secret(cfg.secret.as_bytes()),
            decoding: DecodingKey::from_secret(cfg.secret.as_bytes()),
            cfg,
        }
    }

    /// 签发令牌。
    pub fn encode(&self, claims: &Claims) -> Result<String> {
        encode(&Header::new(Algorithm::HS256), claims, &self.encoding)
            .map_err(|e| Error::Internal(format!("JWT 签发失败: {e}")))
    }

    /// 校验并解析令牌（自动校验过期与签发者）。
    pub fn decode(&self, token: &str) -> Result<Claims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[self.cfg.issuer.as_str()]);
        decode::<Claims>(token, &self.decoding, &validation)
            .map(|data| data.claims)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    Error::Unauthorized("令牌已过期".into())
                }
                jsonwebtoken::errors::ErrorKind::InvalidToken
                | jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                    Error::Unauthorized("令牌无效".into())
                }
                _ => Error::Unauthorized(format!("令牌校验失败: {e}")),
            })
    }

    /// 解析 `Authorization: Bearer <token>` 头。
    pub fn from_authorization(&self, header: Option<&str>) -> Result<Claims> {
        let raw = header.ok_or_else(|| Error::Unauthorized("缺少 Authorization 头".into()))?;
        let token = raw
            .strip_prefix("Bearer ")
            .or_else(|| raw.strip_prefix("bearer "))
            .unwrap_or(raw);
        let claims = self.decode(token)?;
        if claims.ttype != TokenType::Access {
            return Err(Error::Unauthorized("需要访问令牌".into()));
        }
        Ok(claims)
    }

    /// 生成访问+刷新令牌对。
    pub fn issue_pair(
        &self,
        sub: String,
        usr: String,
        role: Option<i32>,
        typ: Option<String>,
    ) -> Result<(String, String, i64)> {
        self.issue_pair_with_shop(sub, usr, role, typ, None)
    }

    /// 生成访问+刷新令牌对（带商户店铺 ID，支持多租户隔离）。
    pub fn issue_pair_with_shop(
        &self,
        sub: String,
        usr: String,
        role: Option<i32>,
        typ: Option<String>,
        shop_id: Option<i32>,
    ) -> Result<(String, String, i64)> {
        self.issue_pair_with_tenant(sub, usr, role, typ, shop_id, None)
    }

    /// 生成访问+刷新令牌对（含店铺/供应商租户 ID）。
    pub fn issue_pair_with_tenant(
        &self,
        sub: String,
        usr: String,
        role: Option<i32>,
        typ: Option<String>,
        shop_id: Option<i32>,
        vendor_id: Option<i32>,
    ) -> Result<(String, String, i64)> {
        let access = self.encode(&Claims::access_with_tenant(
            sub.clone(),
            usr.clone(),
            role,
            typ.clone(),
            shop_id,
            vendor_id,
            &self.cfg,
        ))?;
        let refresh = self.encode(&Claims::refresh_with_tenant(
            sub,
            usr,
            role,
            typ,
            shop_id,
            vendor_id,
            &self.cfg,
        ))?;
        Ok((access, refresh, self.cfg.access_expire_secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svc() -> JwtService {
        JwtService::new(JwtConfig::default())
    }

    #[test]
    fn roundtrip() {
        let s = svc();
        let (access, refresh, _) = s.issue_pair("1".into(), "admin".into(), Some(1), Some("admin".into())).unwrap();
        let c = s.decode(&access).unwrap();
        assert_eq!(c.sub, "1");
        assert_eq!(c.ttype, TokenType::Access);
        let r = s.decode(&refresh).unwrap();
        assert_eq!(r.ttype, TokenType::Refresh);
    }

    #[test]
    fn wrong_token_rejected() {
        let s = svc();
        assert!(s.decode("not-a-token").is_err());
        // 另一个密钥签发的令牌必须被拒绝
        let other = JwtService::new(JwtConfig {
            secret: "other-secret".into(),
            ..JwtConfig::default()
        });
        let forged = other.encode(&Claims::access("1".into(), "hacker".into(), None, None, &JwtConfig::default())).unwrap();
        assert!(s.decode(&forged).is_err());
    }

    #[test]
    fn bearer_header_parse() {
        let s = svc();
        let (access, _, _) = s.issue_pair("7".into(), "u".into(), None, None).unwrap();
        let header = format!("Bearer {access}");
        let c = s.from_authorization(Some(&header)).unwrap();
        assert_eq!(c.sub, "7");
        assert!(s.from_authorization(None).is_err());
    }

    #[test]
    fn multi_tenant_shop_id_claims() {
        let s = svc();
        // Merchant admin with shop_id = 42
        let (access, refresh, _) = s
            .issue_pair_with_shop("10".into(), "merchant1".into(), Some(2), Some("merchant".into()), Some(42))
            .unwrap();
        let c = s.decode(&access).unwrap();
        assert_eq!(c.sub, "10");
        assert_eq!(c.typ.as_deref(), Some("merchant"));
        assert_eq!(c.shop_id, Some(42));

        let r = s.decode(&refresh).unwrap();
        assert_eq!(r.shop_id, Some(42));

        // Platform admin with shop_id = 0
        let (p_access, _, _) = s
            .issue_pair_with_shop("1".into(), "admin".into(), Some(1), Some("admin".into()), Some(0))
            .unwrap();
        let pc = s.decode(&p_access).unwrap();
        assert_eq!(pc.shop_id, Some(0));
    }
}