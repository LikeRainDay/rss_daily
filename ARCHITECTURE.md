# 项目架构说明

## 目录结构

```
rss-daily-cursor/
├── src/
│   ├── github_trending/      # GitHub trending 相关功能
│   │   ├── client.rs         # GitHub API 客户端
│   │   ├── fetcher.rs        # 趋势数据抓取器（含历史管理）
│   │   ├── history.rs        # 历史数据管理
│   │   └── card.rs            # 卡片生成（HTML + 图片）
│   ├── push_post/             # 推送平台支持
│   │   ├── platform.rs       # 平台 trait 定义
│   │   └── csdn.rs            # CSDN 平台实现
│   ├── storage/               # 数据存储
│   │   └── data_storage.rs    # JSON 数据存储管理
│   ├── config.rs              # 配置管理
│   ├── models.rs              # 数据模型
│   ├── rss_gen.rs             # RSS 生成器
│   ├── summary.rs             # 总结生成器（支持 LLM）
│   ├── image_gen.rs           # 图片生成器
│   └── main.rs                # 主程序入口
├── data/                      # 数据存储目录
│   └── github_trending/       # GitHub trending 数据
│       ├── YYYY-MM-DD_trending.json  # 每日数据
│       └── history.json              # 历史记录
├── docs/rss/                  # RSS 输出目录
├── config.toml                # 配置文件
└── .github/workflows/         # GitHub Actions
```

## 核心功能模块

### 1. GitHub Trending (`src/github_trending/`)

#### `client.rs` - GitHub API 客户端
- 负责与 GitHub API 通信
- 支持按语言搜索仓库
- 计算趋势分数（stars、forks、更新时间）

#### `fetcher.rs` - 趋势数据抓取器
- 拉取每日趋势数据
- 自动保存到 `data/github_trending/` 目录
- 管理历史数据
- 过滤已推荐内容
- 智能排序算法

#### `history.rs` - 历史数据管理
- 记录推荐历史
- 支持去重
- 跟踪推荐次数
- 支持重新推荐（如果算法判断值得）

#### `card.rs` - 卡片生成器
- 生成 HTML 格式的仓库卡片
- 支持多语言（中文/英文）
- 集成图片生成
- 自定义样式

### 2. 数据存储 (`src/storage/`)

#### `data_storage.rs` - JSON 数据存储
- 按日期和名字保存数据
- 文件格式：`YYYY-MM-DD_name.json`
- 支持加载历史数据
- 自动创建目录结构

### 3. 推送平台 (`src/push_post/`)

#### `platform.rs` - 平台接口
- 定义统一的推送接口
- 支持批量推送
- 错误处理

#### `csdn.rs` - CSDN 平台实现
- CSDN 登录
- 文章发布
- 支持环境变量配置

### 4. 总结生成 (`src/summary.rs`)

- **简单模式**：基于规则的总结（无需 API）
- **OpenAI 模式**：使用 OpenAI API 生成总结
- **本地模型模式**：支持本地 LLM（如 Ollama）
- **容错机制**：LLM 调用失败时自动回退到简单模式

### 5. 图片生成 (`src/image_gen.rs`)

- 生成精美的卡片图片
- 支持自定义样式（颜色、字体、尺寸）
- 跨平台字体支持
- 多语言文本渲染

## 数据流

```
GitHub API
    ↓
TrendingFetcher.fetch_daily_trending()
    ↓
保存到 data/github_trending/YYYY-MM-DD_trending.json
    ↓
更新 history.json
    ↓
过滤和排序
    ↓
生成卡片（HTML + 图片）
    ↓
生成 RSS Feed
    ↓
（可选）推送到平台（CSDN 等）
```

## 配置说明

### 环境变量

- `GITHUB_TOKEN`: GitHub Personal Access Token（必需）
- `OPENAI_API_KEY`: OpenAI API Key（可选，用于 LLM 总结）
- `CSDN_USERNAME`: CSDN 用户名（可选，用于推送）
- `CSDN_PASSWORD`: CSDN 密码（可选，用于推送）

### config.toml

主要配置项：
- `languages`: 要抓取的语言列表
- `categories`: RSS 分类配置
- `summary`: 总结生成配置
- `image`: 图片生成配置
- `push`: 推送平台配置
- `allow_recommend_again`: 是否允许重新推荐

## 工作流程

1. **定时触发**：GitHub Actions 每 6 小时运行一次
2. **数据抓取**：从 GitHub API 拉取趋势数据
3. **数据存储**：保存到 `data/` 目录（JSON 格式）
4. **历史管理**：更新历史记录，用于去重和排序
5. **内容生成**：
   - 生成总结（支持 LLM）
   - 生成卡片图片
   - 生成 HTML 卡片
6. **RSS 生成**：生成 RSS Feed
7. **平台推送**：（可选）推送到 CSDN 等平台
8. **自动提交**：GitHub Actions 自动提交并推送到仓库

## 扩展性

### 添加新的数据源

1. 在 `src/` 下创建新的目录（如 `src/hackernews/`）
2. 实现类似 `github_trending` 的结构
3. 在主程序中集成

### 添加新的推送平台

1. 在 `src/push_post/` 下创建新文件
2. 实现 `PostPlatform` trait
3. 在 `config.toml` 中添加配置
4. 在主程序中添加平台支持

### 自定义总结算法

修改 `src/summary.rs` 中的总结生成逻辑，或实现新的 LLM 接口。
