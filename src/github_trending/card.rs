use super::summary::SummaryGenerator;
use crate::config::Config;
use crate::models::Repository;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub struct CardGenerator {
    summary_gen: SummaryGenerator,
    html_template: String,
}

impl CardGenerator {
    pub fn new(config: &Config) -> Self {
        // 加载 HTML 模板
        let html_template =
            fs::read_to_string("templates/card_template.html").unwrap_or_else(|_| {
                log::warn!("Failed to load templates/card_template.html, using default");
                Self::default_html_template()
            });

        Self {
            summary_gen: SummaryGenerator::new(config),
            html_template,
        }
    }

    fn default_html_template() -> String {
        // 默认模板作为后备
        include_str!("../../templates/card_template.html").to_string()
    }

    /// 生成仓库卡片（HTML + 图片）
    pub async fn generate_card(
        &self,
        repo: &Repository,
        language: &str,
        output_dir: &Path,
        category: &str,
        config: &Config,
        date: &str,
        rank: usize,
        browser: &headless_chrome::Browser,
    ) -> Result<Card> {
        // 生成总结（独立于 Chrome，确保一定成功）
        let summary = self.summary_gen.generate_summary(repo, language).await?;

        // 生成 HTML（图片路径稍后生成）
        let html = self.generate_html_card(repo, &summary, "", language, rank);

        // 生成图片（从 HTML 转换，包含日期）
        // 使用独立的错误处理，确保 Chrome 超时不影响 summary 和 HTML 的生成
        let image_generator = super::image_gen::ImageGenerator::new(config);
        let image_path = match image_generator
            .generate_card_image(repo, &summary, &html, output_dir, category, date, browser)
            .await
        {
            Ok(path) => {
                log::info!("✅ Successfully generated image for {}", repo.name);
                path
            }
            Err(e) => {
                log::warn!(
                    "⚠️  Failed to generate image for {} (Chrome timeout or error): {}. Continuing with summary and HTML.",
                    repo.name,
                    e
                );
                // 返回空图片路径，但不中断流程
                String::new()
            }
        };

        // 更新 HTML 中的图片路径（如果图片生成成功）
        let html = if !image_path.is_empty() {
            let image_filename = image_path.split('/').last().unwrap_or("");
            html.replace("rss/", &format!("rss/{}", image_filename))
        } else {
            html
        };

        Ok(Card {
            html,
            image_path,
            summary,
        })
    }

    /// 生成 HTML 卡片
    pub fn generate_html_card(
        &self,
        repo: &Repository,
        summary: &crate::models::Summary,
        _image_path: &str,
        language: &str,
        rank: usize,
    ) -> String {
        let (stars_label, forks_label, highlights_label, view_repo_label) = if language == "zh" {
            ("Stars", "Forks", "✨ 项目亮点", "访问仓库 →")
        } else {
            ("Stars", "Forks", "✨ Highlights", "View Repository →")
        };

        // Language color mapping
        let lang_color = match repo
            .language
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .as_str()
        {
            "rust" => "#dea584",
            "python" => "#3572A5",
            "javascript" => "#f1e05a",
            "typescript" => "#3178c6",
            "go" => "#00ADD8",
            "java" => "#b07219",
            "cpp" | "c++" => "#f34b7d",
            "c" => "#555555",
            "swift" => "#ffac45",
            "kotlin" => "#A97BFF",
            _ => "#8b949e",
        };

        // 格式化 highlights
        let highlights_html = summary
            .key_points
            .iter()
            .map(|p| format!("<li>{}</li>", p))
            .collect::<Vec<_>>()
            .join("\n            ");

        // 格式化创建时间
        let created_at = repo.created_at.format("%Y-%m-%d").to_string();

        // 获取描述（优先使用仓库自带描述，如果没有则使用 "No description"）
        let description = repo
            .description
            .as_deref()
            .unwrap_or("No description provided");

        // 生成 QR Code (SVG)
        let code = qrcode::QrCode::new(repo.html_url.as_bytes()).unwrap();
        let qr_svg = code
            .render()
            .min_dimensions(100, 100)
            .quiet_zone(false)
            .dark_color(qrcode::render::svg::Color("#0D1117"))
            .light_color(qrcode::render::svg::Color("#FFFFFF"))
            .build();

        // Determine rank class and text
        let rank_class = match rank {
            1 => "rank-1",
            2 => "rank-2",
            3 => "rank-3",
            _ => "rank-normal",
        };
        let rank_text = format!("#{}", rank);

        // Generate Today Stars Badge HTML
        let today_stars_badge = if let Some(stars) = repo.stars_today {
            format!(
                r#"<div class="today-stars-badge">
            <div class="stars-count">+{}</div>
            <div class="stars-label">Today</div>
        </div>"#,
                stars
            )
        } else {
            String::new()
        };

        // 使用模板替换占位符
        self.html_template
            .replace("{{rank_class}}", rank_class)
            .replace("{{rank_text}}", &rank_text)
            .replace("{{today_stars_badge}}", &today_stars_badge)
            .replace("{{avatar_url}}", &repo.owner.avatar_url)
            .replace("{{owner_login}}", &repo.owner.login)
            .replace("{{repo_url}}", &repo.html_url)
            .replace("{{repo_name}}", &repo.name)
            .replace("{{full_name}}", &repo.full_name)
            .replace("{{stars}}", &repo.stars.to_string())
            .replace("{{stars_label}}", stars_label)
            .replace("{{forks}}", &repo.forks.to_string())
            .replace("{{forks_label}}", forks_label)
            .replace("{{lang_color}}", lang_color)
            .replace("{{language}}", repo.language.as_deref().unwrap_or("N/A"))
            .replace("{{description}}", description)
            .replace("{{created_at}}", &created_at)
            .replace("{{open_issues}}", &repo.open_issues.to_string())
            .replace("{{view_repo_label}}", view_repo_label)
            .replace("{{qrcode}}", &qr_svg)
            .replace("{{source_repo}}", "LikeRainDay/rss_daily")
    }
}

#[derive(Debug, Clone)]
pub struct Card {
    pub html: String,
    pub image_path: String,
    pub summary: crate::models::Summary,
}
