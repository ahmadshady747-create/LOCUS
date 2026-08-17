use crate::template::Template;
use std::collections::HashSet;

pub struct TemplateSearcher;

impl TemplateSearcher {
    pub fn search<'a>(templates: &'a [&'a Template], query: &str) -> Vec<&'a Template> {
        if query.trim().is_empty() {
            return templates.to_vec();
        }

        let query_lower = query.to_lowercase();
        let keywords: HashSet<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(&Template, usize)> = templates
            .iter()
            .filter_map(|t| {
                let score = Self::score_template(t, &keywords);
                if score > 0 {
                    Some((*t, score))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.into_iter().map(|(t, _)| t).collect()
    }

    fn score_template(template: &Template, keywords: &HashSet<&str>) -> usize {
        let mut score = 0;

        let haystack = format!(
            "{} {} {} {} {}",
            template.id,
            template.name,
            template.description,
            template.category,
            template.tags.join(" ")
        ).to_lowercase();

        for kw in keywords {
            if haystack.contains(kw) {
                score += 1;
            }
        }

        if template.name.to_lowercase().contains(&query_lower(keywords)) {
            score += 5;
        }

        if template.category.to_lowercase().contains(&query_lower(keywords)) {
            score += 3;
        }

        score
    }
}

fn query_lower(keywords: &HashSet<&str>) -> String {
    keywords.iter().cloned().collect::<Vec<_>>().join(" ")
}

pub fn search_by_category<'a>(templates: &'a [&'a Template], category: &str) -> Vec<&'a Template> {
    templates
        .iter()
        .filter(|t| t.category == category)
        .copied()
        .collect()
}

pub fn search_by_security_level<'a>(templates: &'a [&'a Template], level: &crate::template::SecurityLevel) -> Vec<&'a Template> {
    templates
        .iter()
        .filter(|t| &t.security_level == level)
        .copied()
        .collect()
}