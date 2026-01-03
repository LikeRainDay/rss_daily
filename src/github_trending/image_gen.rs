use crate::config::{Config, ImageConfig};
use crate::models::Repository;
use anyhow::Result;
use log::info;
use std::fs;
use std::path::Path;

pub struct ImageGenerator {
    config: ImageConfig,
}

impl ImageGenerator {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.image.clone(),
        }
    }

    pub async fn generate_card_image(
        &self,
        repo: &Repository,
        _summary: &crate::models::Summary,
        html_card: &str,
        output_dir: &Path,
        category: &str,
        date: &str,
        browser: &headless_chrome::Browser,
    ) -> Result<String> {
        if !self.config.enabled {
            return Ok(String::new());
        }

        // 直接使用 output_dir (包含日期结构的目录)
        // main.rs 已经设置了 output_dir 为 docs/rss/YYYY/MM-DD

        // 使用 headless Chrome 将 HTML 转换为图片
        let image_path = self
            .html_to_image(html_card, output_dir, category, repo, date, browser)
            .await?;

        info!("Generated image from HTML: {:?}", image_path);

        // 返回文件名（相对路径）
        let image_filename = format!("{}_{}_{}.png", date, category, repo.name.replace("/", "_"));
        Ok(image_filename)
    }

    /// 使用 headless Chrome 将 HTML 转换为图片
    async fn html_to_image(
        &self,
        html_card: &str,
        output_dir: &Path,
        category: &str,
        repo: &Repository,
        date: &str,
        browser: &headless_chrome::Browser,
    ) -> Result<std::path::PathBuf> {
        use headless_chrome::protocol::cdp::Page;
        use std::time::Duration;

        // HTML 已包含完整文档结构
        let full_html = html_card.to_string();

        // 创建临时 HTML 文件
        let temp_dir = std::env::temp_dir();
        let temp_html = temp_dir.join(format!("card_{}_{}.html", category, repo.id));
        fs::write(&temp_html, &full_html)?;

        // create new tab from shared browser
        let tab = browser.new_tab()?;

        // HTML template has its own background defined, no need to override

        // 加载 HTML 文件
        let file_url = format!("file://{}", temp_html.to_str().unwrap());
        tab.navigate_to(&file_url)?.wait_until_navigated()?;

        // 等待页面渲染 (Increased wait time for fonts/images)
        std::thread::sleep(Duration::from_millis(2000));

        // 截图（文件名包含日期）
        let image_filename = format!("{}_{}_{}.png", date, category, repo.name.replace("/", "_"));
        let image_path = output_dir.join(&image_filename);

        // Define clip region
        let clip = Page::Viewport {
            x: 0.0,
            y: 0.0,
            width: self.config.width as f64,
            height: self.config.height as f64,
            scale: 1.0,
        };

        let png_data = tab.capture_screenshot(
            Page::CaptureScreenshotFormatOption::Png,
            None,
            Some(clip),
            true,
        )?;

        // 保存图片
        fs::write(&image_path, png_data)?;

        // 清理临时文件
        let _ = fs::remove_file(&temp_html);

        Ok(image_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Owner, Repository};
    use headless_chrome::Browser;

    #[tokio::test]
    async fn test_generate_card_image() -> Result<()> {
        // 1. Setup Mock Data
        // Load config from file, fallback to default if missing (though user asked for config.toml)
        let config = Config::load().unwrap_or_else(|_| {
            eprintln!("Warning: Failed to load config.toml in test, using default.");
            Config::default()
        });
        let generator = ImageGenerator::new(&config);

        let repo = Repository {
            id: 1,
            name: "test-owner/test-repo".to_string(),
            full_name: "test-owner/test-repo".to_string(),
            description: Some("A test repository for image generation".to_string()),
            html_url: "https://github.com/test-owner/test-repo".to_string(),
            stars: 100,
            forks: 50,
            language: Some("Rust".to_string()),
            topics: vec!["test".to_string(), "rust".to_string()],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            pushed_at: chrono::Utc::now(),
            open_issues: 10,
            owner: Owner {
                login: "test-owner".to_string(),
                avatar_url: "https://github.com/test-owner.png".to_string(),
            },
            readme: None,
            stars_today: Some(10),
        };

        // 2. Prepare Output Directory
        let output_dir = std::path::PathBuf::from("target/test_images");
        if !output_dir.exists() {
            std::fs::create_dir_all(&output_dir)?;
        }

        // 3. Initialize Browser
        let browser_opts = headless_chrome::LaunchOptions::default();
        let browser = Browser::new(browser_opts)?;

        // 4. Load and Populate Template (Manual replacement for test)
        // Note: In real app, CardGenerator handles this. Here we mimic it for ImageGenerator test.
        let template_path = std::path::PathBuf::from("templates/card_template.html");
        let template_content = if template_path.exists() {
            std::fs::read_to_string(template_path)?
        } else {
            // Fallback if running from a different directory or template missing in test env
            r#"<!DOCTYPE html><html><body><h1>Fallback Template</h1></body></html>"#.to_string()
        };

        let html_card = template_content
            .replace("{{rank_class}}", "rank-1")
            .replace("{{rank_text}}", "#1")
            .replace("{{today_stars_badge}}", "")
            .replace("{{avatar_url}}", &repo.owner.avatar_url)
            .replace("{{owner_login}}", &repo.owner.login)
            .replace("{{repo_url}}", &repo.html_url)
            .replace("{{repo_name}}", &repo.name)
            .replace("{{full_name}}", &repo.full_name)
            .replace("{{stars}}", &repo.stars.to_string())
            .replace("{{stars_label}}", "Stars")
            .replace("{{forks}}", &repo.forks.to_string())
            .replace("{{forks_label}}", "Forks")
            .replace("{{lang_color}}", "#dea584") // Rust color
            .replace("{{language}}", "Rust")
            .replace("{{description}}", repo.description.as_ref().unwrap())
            .replace("{{created_at}}", "2026-01-01")
            .replace("{{open_issues}}", "10")
            .replace("{{view_repo_label}}", "View")
            .replace("{{qrcode}}", "") // Skip QR code for simple test
            .replace("{{source_repo}}", "rss-daily");

        // 5. Run Generation
        let result = generator
            .generate_card_image(
                &repo,
                &crate::models::Summary {
                    content: "summary".to_string(),
                    language: "en".to_string(),
                    key_points: vec![],
                },
                &html_card,
                &output_dir,
                "test_real_template",
                "2026-01-01",
                &browser,
            )
            .await;

        // 6. Verify Result
        assert!(result.is_ok());
        let filename = result.unwrap();
        assert!(filename.ends_with(".png"));

        let path = output_dir.join(filename);
        assert!(path.exists());
        println!("Generated test image at: {:?}", path);

        Ok(())
    }
}
