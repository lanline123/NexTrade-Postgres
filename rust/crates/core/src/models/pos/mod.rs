//! pos 域 DTO / 查询参数 / VO。
//!
//! Java 来源：overseas / b2b2b-pro 工程**均无** `PosDeviceController`
//! （全库 grep `pos/device`、`PosDevice` 只有打印模块的 `PrintPosDevice`，
//! 与 `pos_device` 表无关）。本组 DTO 按线上管理端前端契约
//! （`java/view/admin/src/api/setting/cashier.ts` + 表 `pos_device` schema）定义。

pub mod cashier;
pub mod device;
