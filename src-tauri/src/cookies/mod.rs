//! Cookie profile 领域逻辑
//!
//! - `netscape`：Netscape cookies.txt 文本的校验、规范化与域名派生
//! - `matcher`：profile 域名规则与下载链接的匹配（纯函数，可单测）

pub mod matcher;
pub mod netscape;
