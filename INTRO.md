# RSS Daily Cursor

一个基于 Rust 的 GitHub Trending RSS 生成服务，自动抓取 GitHub 热门仓库，生成 RSS Feed 和精美的卡片图片。

## ✨ 特性

- 🚀 **自动抓取**：使用 GitHub API 抓取 trending 仓库（支持匿名访问）
- 💾 **数据存储**：按日期和名字保存 JSON 数据到 `data/` 目录
- 📜 **历史管理**：自动记录推荐历史，支持去重和智能重新推荐
- 📰 **RSS 生成**：自动生成符合标准的 RSS Feed
- 🎨 **卡片生成**：使用 HTML 转图片技术，生成高质量卡片图片
- 📝 **每日 README**：自动生成当天的 README 汇总文档
- 🌍 **多语言支持**：支持中文和英文总结
- 🤖 **LLM 总结**：可选支持 OpenAI/本地模型生成总结（失败自动回退）
- 📊 **智能分类**：按技术栈自动分类（后端、前端、移动端等）
- ⚡ **趋势算法**：基于 stars、forks、更新时间的智能排序
- 🔍 **质量过滤**：自动过滤低星仓库（可配置最小 stars）
- 📤 **平台推送**：支持推送到 CSDN 等平台（可扩展）
- 🔄 **自动化**：GitHub Actions 定时自动更新

## 🏗️ 架构

```
GitHub Actions (定时任务)
    ↓
Rust 服务
    ├── github_trending/        # GitHub trending 模块
    │   ├── client.rs          # GitHub API 客户端
    │   ├── fetcher.rs         # 数据抓取和历史管理
    │   ├── history.rs         # 历史记录管理
    │   ├── card.rs            # 卡片生成（HTML + 图片）
    │   ├── image_gen.rs       # HTML 转图片（headless Chrome）
    │   ├── rss_gen.rs         # RSS 生成器
    │   ├── summary.rs         # 总结生成（支持 LLM）
    │   └── readme_gen.rs      # README 生成器
    ├── storage/                # 数据存储
    │   └── data_storage.rs    # JSON 数据存储
    └── push_post/             # 推送平台
        └── csdn.rs            # CSDN 平台支持
    ↓
data/github_trending/          # 数据存储
    ├── YYYY-MM-DD_trending.json
    └── history.json
    ↓
docs/rss/                      # RSS 输出
    ├── README_YYYY-MM-DD.md   # 每日 README 汇总
    ├── backend.xml            # RSS Feed
    ├── frontend.xml
    ├── mobile.xml
    ├── ai-ml.xml
    └── YYYY-MM-DD_*.png       # 带日期的卡片图片
    ↓
GitHub Pages (公开访问)
    ↓
（可选）推送到 CSDN 等平台
```

## 📦 安装

### 前置要求

- Rust 1.70+ ([安装指南](https://www.rust-lang.org/tools/install))
- Chrome/Chromium（用于 HTML 转图片）
  - macOS: `brew install chromium`
  - Linux: `apt-get install chromium-browser` 或 `yum install chromium`
  - Windows: 自动下载（首次运行）
- GitHub Personal Access Token（可选，但推荐）
  - 无 Token：60 次/小时（匿名访问）
  - 有 Token：5000 次/小时（认证访问）

### 配置

1. 克隆仓库：

```bash
git clone <your-repo-url>
cd rss-daily-cursor
```

2. 配置 GitHub Token（可选，但推荐）：

```bash
# 方式1: 环境变量（推荐）
export GITHUB_TOKEN=your_github_token

# 方式2: .env 文件
echo "GITHUB_TOKEN=your_github_token" > .env

# 方式3: 编辑 config.toml
# 或者不配置，使用匿名访问（速率限制较低）
```

3. （可选）配置推送平台：

```bash
# CSDN
export CSDN_USERNAME=your_csdn_username
export CSDN_PASSWORD=your_csdn_password

# OpenAI（用于 LLM 总结）
export OPENAI_API_KEY=your_openai_api_key
```

4. 编辑 `config.toml` 自定义配置：

```toml
# GitHub API Token (也可以通过环境变量 GITHUB_TOKEN 设置)
github_token = ""

# 要抓取的语言列表
languages = ["java", "rust", "go", "cpp", "c", "swift", "kotlin", "r", "typescript", "javascript"]

# RSS 分类配置 - 只保留每日最有价值的 Top 10
[[categories]]
name = "daily-top"
language = "zh"  # "zh" 或 "en"
keywords = []  # 不限制关键词，包含所有类型
topics = []    # 不限制主题
max_items = 10  # 只推荐前 10 个最有价值的项目

# 总结生成配置
[summary]
enabled = true
provider = "simple"  # "simple", "openai", "local"
api_key = ""  # OpenAI API key (如果使用 OpenAI)

# 图片生成配置
[image]
enabled = true
width = 1200
height = 630
background_color = "#1a1a1a"

# 最小 stars 数量过滤
min_stars = 10

# 趋势算法配置
[algorithm]
significant_growth_threshold = 0.20  # 显著增长阈值（20%）
recency_decay_days = 7.0             # 时间衰减半衰期
new_repo_window_days = 30            # 新仓库识别窗口期
growth_rate_window_days = 7          # 增长率计算窗口期

# 推送配置
[push]
enabled = false
```

## 🚀 使用

### 本地运行

```bash
# 构建
cargo build --release

# 运行
cargo run --release
```

### GitHub Actions 自动运行

1. 在 GitHub 仓库设置中启用 GitHub Pages（选择 `docs` 目录）
2. 确保 `.github/workflows/rss.yml` 已配置
3. Actions 会自动按计划运行（默认每 6 小时）

## 📁 项目结构

```
rss-daily-cursor/
├── src/
│   ├── github_trending/      # GitHub trending 模块
│   │   ├── client.rs         # GitHub API 客户端
│   │   ├── fetcher.rs        # 数据抓取和历史管理
│   │   ├── history.rs        # 历史记录管理
│   │   └── card.rs           # 卡片生成
│   ├── push_post/            # 推送平台支持
│   │   ├── platform.rs       # 平台接口
│   │   └── csdn.rs           # CSDN 实现
│   ├── storage/               # 数据存储
│   │   └── data_storage.rs   # JSON 存储管理
│   ├── main.rs               # 主程序入口
│   ├── config.rs             # 配置管理
│   ├── rss_gen.rs            # RSS 生成器
│   ├── summary.rs            # 总结生成器（支持 LLM）
│   ├── image_gen.rs          # 图片生成器
│   └── models.rs             # 数据模型
├── data/                     # 数据存储目录
│   └── github_trending/      # GitHub trending 数据
│       ├── YYYY-MM-DD_trending.json
│       └── history.json
├── docs/
│   └── rss/                  # RSS 输出目录
│       ├── README_YYYY-MM-DD.md  # 每日 README
│       ├── *.xml             # RSS Feed 文件
│       └── YYYY-MM-DD_*.png  # 卡片图片（带日期）
├── config.toml               # 配置文件
├── Cargo.toml                # Rust 依赖
└── .github/
    └── workflows/
        └── rss.yml           # GitHub Actions 配置
```

## 🔧 配置说明

### 数据存储

所有拉取的数据会自动保存到 `data/github_trending/` 目录：

- 每日数据：`YYYY-MM-DD_trending.json`
- 历史记录：`history.json`（用于去重和排序）

### 分类配置

在 `config.toml` 中配置分类：

```toml
[[categories]]
name = "backend"
language = "zh"  # "zh" 或 "en"
keywords = ["backend", "server", "api"]
topics = ["backend", "api"]
max_items = 20
```

### 历史管理和去重

```toml
# 是否允许重新推荐已推荐过的内容（如果算法判断值得）
allow_recommend_again = true
```

系统会自动：

- 记录推荐历史
- 过滤已推荐内容（除非算法判断值得重新推荐）
- 根据历史数据智能排序

### 总结生成

支持三种模式：

- `simple`: 基于规则的简单总结（无需 API，默认）
- `openai`: 使用 OpenAI API 生成总结（需要 API key）
- `local`: 使用本地模型（需要配置本地服务）

**容错机制**：LLM 调用失败时自动回退到简单模式，不影响 RSS 生成。

### 图片生成

使用 **HTML 转图片**技术（headless Chrome），完美支持 HTML/CSS 渲染：

```toml
[image]
enabled = true
width = 1200
height = 630
background_color = "#1a1a1a"
text_color = "#ffffff"
font_size = 24
```

**特点**：

- ✅ 完美支持 HTML/CSS
- ✅ 支持中文字体和 emoji
- ✅ 图片文件名包含日期：`YYYY-MM-DD_category_repo.png`
- ✅ 与 RSS 中的 HTML 卡片保持一致

### 最小 Stars 过滤

过滤低星仓库，只显示真正受欢迎的项目：

```toml
# 最小 stars 数量过滤（默认 10）
min_stars = 10
```

### 每日 README 生成

自动生成当天的 README 汇总文档：

- 文件：`docs/rss/README_YYYY-MM-DD.md`
- 包含：统计信息、分类列表、仓库详情、RSS 链接
- 格式：Markdown，可直接在 GitHub 查看

### 推送平台配置

```toml
[push]
enabled = true  # 启用推送功能

[[push.platforms]]
name = "csdn"
# username 和 password 可以通过环境变量设置
```

## 📡 RSS Feed 地址

部署到 GitHub Pages 后，RSS Feed 地址：

**每日 Top 10 推荐：**

- `https://your-username.github.io/rss-daily-cursor/rss/daily-top.xml`

> 这个 feed 包含当日根据趋势算法排序后最有价值的前 10 个项目，涵盖所有技术栈。

## 📝 每日 README

每天会自动生成 README 汇总文档：

- `https://your-username.github.io/rss-daily-cursor/rss/README_YYYY-MM-DD.md`

包含当天的所有热门仓库、统计信息和 RSS 链接。

## 🎯 趋势算法

项目使用改进的智能趋势评分算法，兼顾新老项目的公平性：

### 核心算法

```
score = log(stars + 1) × 3.0
      + log(forks + 1) × 2.0
      + growth_rate_score × 100.0
      + recency_score × 50.0
      + new_repo_bonus
```

### 算法优势

- **对数缩放** (`log(stars + 1)`): 降低绝对值影响，让新项目有机会与知名项目竞争
- **增长率评分**: 基于历史数据计算 7 天内的 stars 增长率，捕捉真正的"趋势"
- **时间衰减因子**: 使用指数衰减 (`exp(-days/7)`，7 天半衰期)，优先推荐活跃项目
- **新项目加分**: 创建 30 天内的项目获得额外加分，鼓励探索新内容

### 增长率计算

系统会根据历史推荐记录计算增长情况：

| 增长率 | 评分              | 说明                       |
| ------ | ----------------- | -------------------------- |
| > 20%  | growth_rate × 100 | 显著增长，重点推荐         |
| 0-20%  | growth_rate × 50  | 稳定增长，适度加分         |
| ≤ 0%   | -30 分            | 已推荐但无增长，降低优先级 |
| 新项目 | +20 分            | 首次推荐，优先展示         |

### 历史数据管理

系统会考虑历史推荐记录：

- **"显著增长"定义**: 7 天内 stars 增长率 ≥ 20%
- **重新推荐策略**: 已推荐项目如果有显著增长，会重新进入推荐列表
- **去重机制**: 无增长的已推荐项目会被降低优先级

### 可配置参数

在 `config.toml` 中调整算法参数：

```toml
[algorithm]
significant_growth_threshold = 0.20  # 显著增长阈值（20%）
recency_decay_days = 7.0             # 时间衰减半衰期
new_repo_window_days = 30            # 新仓库识别窗口期
growth_rate_window_days = 7          # 增长率计算窗口期
```

## 🔌 扩展

### 添加新的数据源

在 `src/github_trending/` 目录下创建新的模块，参考现有结构。

### 自定义总结模板

修改 `src/github_trending/summary.rs` 中的 `generate_chinese_summary` 和 `generate_english_summary` 方法。

### 集成 AI 总结

在 `src/github_trending/summary.rs` 中实现 `generate_openai_summary` 方法，调用 OpenAI API。

### 自定义 README 格式

修改 `src/github_trending/readme_gen.rs` 中的 `generate_daily_readme` 方法。

### 自定义卡片样式

卡片的 HTML 和 CSS 已独立为模板文件，方便自定义：

**模板文件位置：**

- `templates/card.html` - HTML 结构模板
- `templates/card_style.css` - CSS 样式表

**自定义步骤：**

1. 编辑 `templates/card_style.css` 修改颜色、字体、间距等
2. 编辑 `templates/card.html` 调整布局结构
3. 模板支持占位符（如 `{{repo_name}}`, `{{stars}}` 等）
4. 修改后重新运行程序即可生效

**示例：修改渐变背景色**

```css
.repo-card {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  /* 改为你喜欢的渐变色 */
}
```

## 📝 License

MIT License

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📋 输出文件说明

### 数据文件（`data/github_trending/`）

- `YYYY-MM-DD_trending.json` - 每日趋势数据
- `history.json` - 推荐历史记录

### RSS 文件（`docs/rss/`）

- `README_YYYY-MM-DD.md` - 每日 README 汇总
- `{category}.xml` - RSS Feed 文件
- `YYYY-MM-DD_{category}_{repo}.png` - 卡片图片（带日期）

### 文件命名规则

- 图片：`{日期}_{分类}_{仓库名}.png`
- README：`README_{日期}.md`
- 数据：`{日期}_trending.json`

## ❓ 常见问题

### Chrome/Chromium 未找到

**错误信息**: `Failed to launch browser` 或 `Chrome not found`

**解决方案**:

```bash
# macOS
brew install chromium

# Ubuntu/Debian
sudo apt-get install chromium-browser

# CentOS/RHEL
sudo yum install chromium
```

### GitHub API 速率限制

**错误信息**: `API rate limit exceeded`

**解决方案**:

1. 配置 GitHub Personal Access Token（提升至 5000 次/小时）
2. 减少抓取频率
3. 使用 `min_stars` 过滤减少数据量

### RSS 生成失败

**错误信息**: `Failed to generate RSS` 或权限错误

**解决方案**:

1. 确保 `docs/rss/` 目录有写权限
2. 检查磁盘空间是否充足
3. 查看日志文件定位具体错误

### 卡片图片不显示

**可能原因**: 图片生成失败或路径错误

**解决方案**:

1. 确认 Chrome/Chromium 已正确安装
2. 检查 `docs/rss/` 目录下是否有 PNG 文件
3. 查看日志中的图片生成错误信息

### 模板文件加载失败

**警告信息**: `Failed to load templates/card.html`

**解决方案**:

1. 确保在项目根目录运行程序
2. 检查 `templates/` 目录是否存在
3. 如果文件缺失，程序会使用内置默认模板

## 📧 联系方式

如有问题，请提交 Issue。
