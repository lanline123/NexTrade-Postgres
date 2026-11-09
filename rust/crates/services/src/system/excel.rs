//! Excel 导出/导入基础设施（替代 Java `ExcelUtils` / `@ResponseExcel` / EasyExcel）。
//!
//! - 导出：`build_xlsx(headers, rows, sheet)` → xlsx 字节流（前端下载）；
//! - 导入：`read_xlsx(bytes)` → 二维字符串（首行为表头，由调用方映射）。
//!
//! 列顺序/表头与 Java `@ExcelProperty(...)` 保持一致，调用方负责拼中文表头。
use std::io::Cursor;

use calamine::Reader;
use rust_xlsxwriter::Workbook;

use nextrade_core::{Error, Result};

/// 生成 xlsx 字节流（首行为表头）。
///
/// # Errors
/// 列号越界 / 写入失败。
pub fn build_xlsx(headers: &[&str], rows: &[Vec<String>], sheet_name: &str) -> Result<Vec<u8>> {
    build_xlsx_with_leading(headers, None, rows, sheet_name)
}

/// 生成 xlsx，可选在工作表首行写一行说明（其后是表头与数据）。
///
/// 批量发货模板即「说明行 + 表头 + 数据行」三段式。
pub fn build_xlsx_with_leading(
    headers: &[&str],
    leading: Option<&[String]>,
    rows: &[Vec<String>],
    sheet_name: &str,
) -> Result<Vec<u8>> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet
        .set_name(sheet_name)
        .map_err(|e| Error::Internal(format!("Excel 工作表命名失败: {e}")))?;
    let mut header_row = 0u32;
    if let Some(leading) = leading {
        for (col, value) in leading.iter().enumerate() {
            if value.is_empty() {
                continue;
            }
            let col = u16::try_from(col).map_err(|_| Error::Internal("Excel 列数过多".into()))?;
            sheet
                .write_string(0, col, value.as_str())
                .map_err(|e| Error::Internal(format!("Excel 说明行写入失败: {e}")))?;
        }
        header_row = 1;
    }
    for (col, header) in headers.iter().enumerate() {
        let col = u16::try_from(col).map_err(|_| Error::Internal("Excel 列数过多".into()))?;
        sheet
            .write_string(header_row, col, *header)
            .map_err(|e| Error::Internal(format!("Excel 表头写入失败: {e}")))?;
    }
    for (row_idx, row) in rows.iter().enumerate() {
        let row_idx = u32::try_from(row_idx)
            .map_err(|_| Error::Internal("Excel 行数过多".into()))?
            + header_row
            + 1;
        for (col, value) in row.iter().enumerate() {
            if value.is_empty() {
                continue;
            }
            let col = u16::try_from(col).map_err(|_| Error::Internal("Excel 列数过多".into()))?;
            sheet
                .write_string(row_idx, col, value.as_str())
                .map_err(|e| Error::Internal(format!("Excel 单元格写入失败: {e}")))?;
        }
    }
    workbook
        .save_to_buffer()
        .map_err(|e| Error::Internal(format!("Excel 生成失败: {e}")))
}

/// 读取 xlsx 首个工作表为二维字符串（空单元格为空串）。
///
/// # Errors
/// 文件损坏 / 非 xlsx。
pub fn read_xlsx(bytes: &[u8]) -> Result<Vec<Vec<String>>> {
    let cursor = Cursor::new(bytes.to_vec());
    let mut workbook = calamine::open_workbook_auto_from_rs(cursor)
        .map_err(|e| Error::Validation(format!("Excel 解析失败: {e}")))?;
    let range = workbook
        .worksheet_range_at(0)
        .ok_or_else(|| Error::Validation("Excel 缺少工作表".into()))?
        .map_err(|e| Error::Validation(format!("Excel 读取失败: {e}")))?;
    Ok(range
        .rows()
        .map(|row| row.iter().map(cell_to_string).collect())
        .collect())
}

fn cell_to_string(cell: &calamine::Data) -> String {
    match cell {
        calamine::Data::Empty => String::new(),
        calamine::Data::String(v) => v.clone(),
        calamine::Data::Float(v) => {
            if v.fract() == 0.0 {
                format!("{}", *v as i64)
            } else {
                v.to_string()
            }
        }
        calamine::Data::Int(v) => v.to_string(),
        calamine::Data::Bool(v) => v.to_string(),
        calamine::Data::DateTime(v) => v.to_string(),
        calamine::Data::DateTimeIso(v) | calamine::Data::DurationIso(v) => v.clone(),
        calamine::Data::Error(v) => format!("{v:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xlsx_roundtrip() {
        let headers = ["商品编码", "SKU编码", "下限", "上限"];
        let rows = vec![
            vec!["SN001".into(), "SKU001".into(), "1".into(), "10".into()],
            vec!["SN002".into(), "".into(), "2".into(), "20".into()],
        ];
        let bytes = build_xlsx(&headers, &rows, "模板").expect("生成 xlsx");
        assert!(bytes.starts_with(b"PK"), "xlsx 应为 zip 容器");
        let read = read_xlsx(&bytes).expect("解析 xlsx");
        assert_eq!(read[0], headers.map(String::from).to_vec());
        assert_eq!(read[1][0], "SN001");
        assert_eq!(read[1][3], "10");
        assert_eq!(read.len(), 3);
    }

    #[test]
    fn read_rejects_garbage() {
        assert!(read_xlsx(b"not an xlsx").is_err());
    }
}
