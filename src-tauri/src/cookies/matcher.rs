//! Cookie profile 域名规则与下载链接的匹配
//!
//! 匹配语义刻意保持简单且可解释：
//! - 规则是「host 后缀」：`example.com` 命中 `example.com` 与其任意子域
//! - 前缀 `*.` 与协议、路径、大小写、前导点都会被归一化掉
//! - 命中多个 profile 时，规则更具体者优先，其次按用户排序，最后按 id 保证确定性

use url::Url;

/// 参与匹配的 profile 摘要
pub struct MatchCandidate<'a> {
    pub id: &'a str,
    pub domains: &'a [String],
    pub sort_order: i64,
}

/// 匹配结果
pub struct MatchedProfile<'a> {
    pub id: &'a str,
    /// 命中的那条规则
    pub domain: &'a str,
}

/// 从下载链接中取出用于匹配的 host
pub fn extract_host(url: &str) -> Option<String> {
    let parsed = Url::parse(url.trim()).ok()?;
    let host = parsed
        .host_str()?
        .trim_end_matches('.')
        .to_ascii_lowercase();
    (!host.is_empty()).then_some(host)
}

/// 归一化单条域名规则，非法输入返回 `None`
pub fn normalize_domain(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    // 同时接受裸域名与完整链接：交给 Url 统一处理 IDN、端口、路径与大小写
    let candidate = trimmed.trim_start_matches("*.").trim_start_matches('.');
    let target = if candidate.contains("://") {
        candidate.to_string()
    } else {
        format!("http://{candidate}")
    };
    let host = Url::parse(&target).ok()?.host_str()?.to_string();
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() {
        return None;
    }
    Some(host.strip_prefix("www.").unwrap_or(&host).to_string())
}

/// 批量归一化并去重、排序（保证 profile 落库内容稳定）
pub fn normalize_domains(rules: &[String]) -> Vec<String> {
    let mut normalized: Vec<String> = rules
        .iter()
        .filter_map(|rule| normalize_domain(rule))
        .collect();
    normalized.sort();
    normalized.dedup();
    normalized
}

/// 规则是否命中给定 host
pub fn domain_matches(host: &str, rule: &str) -> bool {
    host == rule || host.ends_with(&format!(".{rule}"))
}

/// 在候选 profile 中挑选命中的一个
pub fn match_profile<'a>(
    candidates: &[MatchCandidate<'a>],
    url: &str,
) -> Option<MatchedProfile<'a>> {
    let host = extract_host(url)?;
    let mut hits: Vec<(usize, i64, &'a str, &'a str)> = Vec::new();
    for candidate in candidates {
        for rule in candidate.domains {
            if domain_matches(&host, rule) {
                hits.push((
                    rule.len(),
                    candidate.sort_order,
                    candidate.id,
                    rule.as_str(),
                ));
            }
        }
    }
    // 规则更长（更具体）优先 → 排序更靠前优先 → id 字典序保证结果确定
    hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(b.2)));
    hits.into_iter()
        .next()
        .map(|(_, _, id, domain)| MatchedProfile { id, domain })
}

#[cfg(test)]
mod tests {
    use super::{extract_host, match_profile, normalize_domain, normalize_domains, MatchCandidate};

    fn candidate<'a>(id: &'a str, domains: &'a [String], sort_order: i64) -> MatchCandidate<'a> {
        MatchCandidate {
            id,
            domains,
            sort_order,
        }
    }

    #[test]
    fn extract_host_drops_scheme_path_port_and_case() {
        assert_eq!(
            extract_host("https://WWW.YouTube.com:8443/watch?v=1").as_deref(),
            Some("www.youtube.com")
        );
        assert_eq!(extract_host("not a url"), None);
    }

    #[test]
    fn normalize_domain_accepts_common_forms() {
        for raw in [
            "youtube.com",
            "*.youtube.com",
            ".youtube.com",
            "https://www.youtube.com/feed/subscriptions",
            "  WWW.YouTube.COM  ",
        ] {
            assert_eq!(
                normalize_domain(raw).as_deref(),
                Some("youtube.com"),
                "raw={raw}"
            );
        }
        assert_eq!(normalize_domain("").as_deref(), None);
    }

    #[test]
    fn normalize_domains_sorts_and_dedupes() {
        let rules = vec![
            "Youtube.com".to_string(),
            "*.youtube.com".to_string(),
            "bilibili.com".to_string(),
        ];
        assert_eq!(
            normalize_domains(&rules),
            vec!["bilibili.com".to_string(), "youtube.com".to_string()]
        );
    }

    #[test]
    fn matches_subdomains_but_not_lookalikes() {
        let domains = vec!["youtube.com".to_string()];
        let candidates = vec![candidate("a", &domains, 0)];
        assert_eq!(
            match_profile(&candidates, "https://m.youtube.com/watch?v=1").map(|m| m.id),
            Some("a")
        );
        assert!(match_profile(&candidates, "https://evilyoutube.com/").is_none());
        assert!(match_profile(&candidates, "https://youtube.com.evil.net/").is_none());
    }

    #[test]
    fn prefers_more_specific_rule_then_sort_order() {
        let generic = vec!["youtube.com".to_string()];
        let specific = vec!["music.youtube.com".to_string()];
        let candidates = vec![
            candidate("generic", &generic, 0),
            candidate("specific", &specific, 9),
        ];
        assert_eq!(
            match_profile(&candidates, "https://music.youtube.com/").map(|m| m.id),
            Some("specific")
        );

        let same = vec!["youtube.com".to_string()];
        let ordered = vec![candidate("later", &same, 5), candidate("earlier", &same, 1)];
        assert_eq!(
            match_profile(&ordered, "https://youtube.com/").map(|m| m.id),
            Some("earlier")
        );
    }

    #[test]
    fn returns_none_without_match() {
        let domains = vec!["youtube.com".to_string()];
        let candidates = vec![candidate("a", &domains, 0)];
        assert!(match_profile(&candidates, "https://example.com/video").is_none());
        assert!(match_profile(&candidates, "mailto:someone@example.com").is_none());
    }
}
