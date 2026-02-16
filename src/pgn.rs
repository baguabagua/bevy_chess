pub fn parse_pgn(pgn: &str) -> Vec<String> {
    pgn.split_whitespace()
        .filter_map(|token| {
            // 检查是否是 "{ply}.{step}" 格式
            if let Some(dot_pos) = token.find('.') {
                // 确保点号不在开头或结尾
                if dot_pos > 0 && dot_pos < token.len() - 1 {
                    // 返回点号后面的部分
                    return Some(token[dot_pos + 1..].to_string());
                }
            } else {
                return Some(token.to_string());
            }
            
            None
        })
        .collect()
}