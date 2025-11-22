use crate::Result;
use std::collections::HashMap;

#[derive(Debug)]
pub struct LoginParams {
    pub salt: String,
    pub execution: String,
    pub lt: String,
    pub event_id: String,
    pub rm_shown: Option<String>,
    pub other_params: HashMap<String, String>,
}

/// 解析登录页面 HTML，提取加密所需的 Salt 和表单字段
pub fn parse_login_page(html: &str) -> Result<LoginParams> {
    unimplemented!("Not implemented yet")
}

