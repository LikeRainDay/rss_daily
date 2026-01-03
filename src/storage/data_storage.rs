use crate::models::Repository;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyData {
    pub date: String,
    pub name: String,
    pub repositories: Vec<Repository>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct DataStorage {
    base_dir: PathBuf,
}

impl DataStorage {
    pub fn new(base_path: &str) -> Result<Self> {
        let base_dir = PathBuf::from(base_path);
        std::fs::create_dir_all(&base_dir)?;
        Ok(Self { base_dir })
    }

    /// 保存每日数据
    pub fn save_daily_data(&self, date: &str, name: &str, repos: &[Repository]) -> Result<PathBuf> {
        let data = DailyData {
            date: date.to_string(),
            name: name.to_string(),
            repositories: repos.to_vec(),
            created_at: Utc::now(),
        };

        // 解析日期，构建目录结构: base_dir/YYYY/MM-DD/name.json
        // date 格式预期为 YYYY-MM-DD
        let parts: Vec<&str> = date.split('-').collect();
        let (year, month_day) = if parts.len() == 3 {
            (parts[0], format!("{}-{}", parts[1], parts[2]))
        } else {
            // Fallback
            ("unknown", date.to_string())
        };

        // 创建年份和日期目录
        let archive_dir = self.base_dir.join(year).join(&month_day);
        std::fs::create_dir_all(&archive_dir)?;

        // 文件名格式：name.json (或者保持 date_name.json，用户仅要求目录结构)
        // 用户要求: @[data/github_trending] 下的数据也要按照日期目录啦进行归档/2026/01-02/ 目录结构
        let filename = format!("{}_{}.json", date, name);
        let file_path = archive_dir.join(&filename);

        let content = serde_json::to_string_pretty(&data)?;
        std::fs::write(&file_path, content)?;

        Ok(file_path)
    }

    /// 加载指定日期的数据
    pub fn load_daily_data(&self, date: &str, name: &str) -> Result<DailyData> {
        let filename = format!("{}_{}.json", date, name);
        let file_path = self.base_dir.join(&filename);

        let content = std::fs::read_to_string(&file_path)?;
        let data: DailyData = serde_json::from_str(&content)?;

        Ok(data)
    }

    /// 列出所有日期数据
    pub fn list_dates(&self) -> Result<Vec<String>> {
        let mut dates = Vec::new();

        for entry in std::fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.ends_with(".json") {
                    // 提取日期部分（YYYY-MM-DD）
                    if let Some(date_part) = filename.split('_').next() {
                        if !dates.contains(&date_part.to_string()) {
                            dates.push(date_part.to_string());
                        }
                    }
                }
            }
        }

        dates.sort();
        Ok(dates)
    }

    /// 加载所有历史数据
    pub fn load_all_history(&self) -> Result<Vec<Repository>> {
        let mut all_repos = Vec::new();

        for entry in std::fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(data) = serde_json::from_str::<DailyData>(&content) {
                        all_repos.extend(data.repositories);
                    }
                }
            }
        }

        Ok(all_repos)
    }
}
