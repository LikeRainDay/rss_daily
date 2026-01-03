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
        use headless_chrome::protocol::cdp::Emulation::SetDefaultBackgroundColorOverride;
        use headless_chrome::protocol::cdp::Page;
        use headless_chrome::protocol::cdp::DOM::RGBA;
        use std::time::Duration;

        // HTML 已包含完整文档结构
        let full_html = html_card.to_string();

        // 创建临时 HTML 文件
        let temp_dir = std::env::temp_dir();
        let temp_html = temp_dir.join(format!("card_{}_{}.html", category, repo.id));
        fs::write(&temp_html, &full_html)?;

        // create new tab from shared browser
        let tab = browser.new_tab()?;

        // Enable transparency
        tab.call_method(SetDefaultBackgroundColorOverride {
            color: Some(RGBA {
                r: 0,
                g: 0,
                b: 0,
                a: Some(0.0),
            }),
        })?;

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
