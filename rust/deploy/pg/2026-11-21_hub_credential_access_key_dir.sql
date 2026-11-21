-- 平台联邦修复：hub_credential 唯一索引由 (access_key) 改为 (access_key, direction)。
-- 背景：设计 §4.2 表 1 口径——同一把钥在签发方记入站行（direction=1，只存哈希）、
-- 在持有方记出站行（direction=2，存加密形态），「两侧各存一行、access_key 相同」。
-- 双实例部署时两行分属两库不相见；单实例回环（同扮中枢+成员）时两行同库，
-- 单列唯一索引使配对第二步必撞 uk_hub_credential_access_key（验收 §3 实证）。
-- 改复合后：同方向内 access_key 仍全局唯一（鉴权按钥+入站方向唯一定位），
-- 跨方向孪生行可共存，各行生命周期独立（停用/删除/重置互不影响）。
-- 存量安全：旧索引保证 access_key 全局唯一，(access_key, direction) 必然无重复，
-- 本迁移对任何满足旧约束的数据都可直接执行；现网 hub_credential 为空表更无虞。
-- 幂等：DROP IF EXISTS + 按名重建，可重复执行。生产 auto_migrate=false，部署时手动执行。
BEGIN;

DROP INDEX IF EXISTS uk_hub_credential_access_key;
CREATE UNIQUE INDEX uk_hub_credential_access_key ON hub_credential (access_key, direction);

COMMIT;
