# OmniGet 平台提取器移植 — 交接文档

> 日期：2026-07-01  
> 状态：Phase 0-2 完成，12/15 平台已移植到 `omniget-core`  
> Fork：https://github.com/gkd2323c/omniget  
> PR：https://github.com/tonhowtf/omniget/pull/163

---

## 1. 背景

OmniGet 的原生平台提取器（YouTube、Twitter、Bilibili 等）原本全部位于 `src-tauri/src/platforms/`，与 Tauri GUI 框架强耦合。这导致：

- CLI 工具（`omniget-cli`）无法复用这些提取器，只能降级走通用 yt-dlp 路径
- 桌面应用换框架（如从 Tauri 换成 egui、终端 UI）需要全部重写
- 无法在服务端/无 GUI 场景复用下载逻辑

**目标**：将平台提取器从 Tauri 层迁移到 `omniget-core`，使其成为框架无关的纯 Rust 库。

---

## 2. 架构设计

### 2.1 移植前后对比

```
移植前（Tauri 专属）
├── src-tauri/
│   ├── src/
│   │   ├── platforms/          ← 所有提取器都在这里
│   │   │   ├── youtube/mod.rs
│   │   │   ├── twitter/mod.rs
│   │   │   └── ...
│   │   ├── models/media.rs     ← Tauri 特有模型
│   │   ├── core/               ← 重新导出 omniget_core::
│   │   └── cookies/             ← Tauri 特有 Cookie 管理
│   └── Cargo.toml

移植后（核心库 + Tauri 适配层）
├── src-tauri/
│   ├── omniget-core/
│   │   ├── src/
│   │   │   ├── platforms/      ← 提取器在这里（框架无关）
│   │   │   │   ├── mod.rs         (registry + re-exports)
│   │   │   │   ├── traits.rs      (PlatformDownloader trait)
│   │   │   │   ├── generic_ytdlp.rs
│   │   │   │   ├── youtube.rs
│   │   │   │   ├── twitter.rs     (未移植)
│   │   │   │   └── ...
│   │   │   ├── models/media.rs   ← 核心模型（与 Tauri 共享）
│   │   │   └── core/             ← 下载引擎、HTTP、编解码
│   │   └── Cargo.toml
│   └── src/
│       ├── platforms/          ← 重新导出核心库（pub use）
│       └── ...
```

### 2.2 PlatformDownloader Trait

所有平台提取器实现同一个 trait：

```rust
// omniget-core/src/platforms/traits.rs
#[async_trait]
pub trait PlatformDownloader: Send + Sync {
    fn name(&self) -> &str;
    fn can_handle(&self, url: -> bool;
    async fn get_media_info(&self, url: &str) -> Result<MediaInfo>;
    async fn download(&self, info: &MediaInfo, opts: &DownloadOptions, 
                      progress: Sender<ProgressUpdate>) -> Result<DownloadResult>;
}
```

### 2.3 共享模型

`MediaInfo`、`DownloadOptions`、`DownloadResult`、`MediaType`、`VideoQuality` 定义在 `omniget-core/src/models/media.rs`，Tauri 层通过 `pub use omniget_core::models::media::*` 重新导出，零重复。

---

## 3. 已移植平台清单

### 3.1 核心基础设施

| 组件 | 文件 | 说明 |
|------|------|------|
| PlatformDownloader trait | `omniget-core/src/platforms/traits.ts` | 统一下载接口 |
| GenericYtdlpDownloader | `omniget-core/src/platforms/generic_ytdlp.rs` | yt-dlp 通用封装，含质量选择、HLS、直链、fallback |
| Platform 枚举 | `omniget-core/src/platforms/mod.rs` | URL→平台检测 |
| is_direct_file_url | `omniget-core/src/platforms/mod.rs` | 直链文件检测 |

### 3.2 平台提取器

| 平台 | 文件 | 大小 | 特殊依赖 |
|------|------|------|----------|
| YouTube | `youtube.rs` | 16KB | yt-dlp 引擎、播放列表支持 |
| Vimeo | `vimeo.rs` | 6KB | yt-dlp |
| Douyin | `douyin.rs` | 9KB | yt-dlp + referer |
| Bluesky | `bluesky.rs` | 14KB | GenericYtdlp |
| Twitch | `twitch.rs` | 12KB | GenericYtdlp |
| Pinterest | `pinterest.rs` | 11KB | cookie + referer |
| Reddit | `reddit.rs` | 28KB | cookie + 音频提取 |
| Instagram | `instagram.rs` | 36KB | cookie + GraphQL API |
| TikTok | `tiktok.rs` | 24KB | cookie + 浏览器指纹 |
| P2P | `p2p.rs` + `p2p_words.rs` | 16KB | 纯网络传输 |
| DirectFile | `direct_file.rs` | 5KB | HTTP 直链下载 |

---

## 4. 未移植平台分析

### 4.1 Twitter（~41KB） — Phase 3

**阻塞依赖**：
- `crate::cookies::account_path_for_consumer` — 多账户 Cookie 路径
- `crate::storage::config::load_settings_standalone` — 读取 PlaybackQuality

**移植方案**：
1. 在 `omniget-core` 中定义 `CookieProvider` trait：
   ```rust
   pub trait CookieProvider: Send + Sync {
       fn cookie_path_for(&self, domain: &str) -> Option<PathBuf>;
   }
   ```
2. CLI 实现：返回 `~/.omniget/cookies/cookies.txt`（如果存在）
3. Tauri 实现：委托给 `crate::cookies::account_path_for_consumer`

### 4.2 Bilibili（~200KB/36 文件） — Phase 4-5

**阻塞依赖**：
- Twitter 的两个依赖 +
- WBI 签名算法（`src-tauri/src/platforms/bilibili/wbi.rs`）
- 认证模块（auth/：二维码、短信、CAPTCHA）
- 弹幕系统（danmaku/：ASS/XML/JSON 渲染）
- 番剧/课程解析器（parser/ 8 个子模块）

**移植方案**：
1. 先解决 CookieProvider/SettingsProvider
2. 将 wbi.rs、cookie.rs 等辅助模块移到 `omniget-core/src/platforms/bilibili/`
3. 保持 `mod.rs` 作为入口，子模块按功能分组
4. 弹幕渲染可保留为可选 feature（`features = ["danmaku"]`）

### 4.3 Magnet — 视需求决定

**阻塞依赖**：`librqbit` crate、`crate::core::trackers`

**可选方案**：
- 将 `librqbit` 加为 `omniget-core` 的可选 dependency
- 或保留在 Tauri 层，CLI 不支持磁力下载

---

## 5. 移植模式（标准操作程序）

每个平台的移植遵循相同模式：

```bash
# 1. 在核心库创建新文件
cp src-tauri/src/platforms/<name>/mod.rs src-tauri/omniget-core/src/platforms/<name>.rs

# 2. 替换第一行
sed -i 's/use omniget_core::models::progress::ProgressUpdate;/use crate::models::progress::ProgressUpdate;/' \
  src-tauri/omniget-core/src/platforms/<name>.rs

# 3. 在核心库 platforms/mod.rs 注册
echo "pub mod <name>;" >> src-tauri/omniget-core/src/platforms/mod.rs
echo "pub use <name>::<Name>Downloader;" >> src-tauri/omniget-core/src/platforms/mod.rs

# 4. 更新 Tauri 重新导出
# 在 src-tauri/src/platforms/mod.rs 中：
#   - 删除 `pub mod <name>;`
#   - 添加 `pub use omniget_core::platforms::<Name>Downloader;`

# 5. 更新 lib.rs 引用
# 将 `platforms::<name>::<Name>Downloader` 改为 `omniget_core::platforms::<Name>Downloader`

# 6. 编译测试
cargo check -p omniget-core
cargo check  # 全 workspace
```

### 关键注意点

1. **`pub use` 不要立即删除**：先添加核心库导出，确认编译通过后再清理旧模块
2. **Tauri 特有依赖**：如遇到 `crate::cookies::*` 或 `crate::storage::*`，需要引入 trait 抽象
3. **测试移植**：`#[cfg(test)]` 模块随文件一起搬运，确保单元测试不中断
4. **文件后缀**：核心库使用单文件（`youtube.rs`），不使用目录（`youtube/mod.rs`）

---

## 6. 测试指南

### 6.1 编译检查

```bash
# 仅核心库
cargo check -p omniget-core

# 全 workspace（含 Tauri）
cargo check

# CLI
cargo build -p omniget-cli --release
```

### 6.2 单元测试

```bash
# 核心库全部测试
cargo test -p omniget-core --lib

# 各平台子模块测试
cargo test -p omniget-core --lib platforms::youtube::tests
cargo test -p omniget-core --lib platforms::instagram::tests
```

当前：**191 tests passed**。

### 6.3 CLI 实操测试

```bash
# 构建
cargo build -p omniget-cli --release

# YouTube 信息
omniget-cli --proxy http://127.0.0.1:7897 --json info \
  'https://www.youtube.com/watch?v=dQw4w9WgXcQ'

# YouTube 下载
omniget-cli --proxy http://127.0.0.1:7897 --json download \
  'https://www.youtube.com/watch?v=dQw4w9WgXcQ' \
  -q 1080 -o ~/Downloads

# Cookie 导入
omniget-cli import-cookies ~/Downloads/cookies.txt --dry-run
omniget-cli import-cookies ~/Downloads/cookies.txt -n bilibili

# 批量下载
omniget-cli --proxy http://127.0.0.1:7897 batch urls.txt -m 3
```

### 6.4 回归测试

每次移植新平台后，确认：
- [ ] `cargo check` 无新增错误
- [ ] `cargo test -p omniget-core --lib` 无新增失败
- [ ] 桌面 app 启动正常、下载队列工作正常
- [ ] CLI info/download 对已移植站点正常

---

## 7. 构建产物

| 产物 | 路径 | 说明 |
|------|------|------|
| 核心库（静态） | `target/release/omniget_core.lib` | 平台提取器 + yt-dlp + HTTP |
| 桌面应用 | `target/release/omniget.exe` | GUI + 所有平台 |
| CLI 工具 | `target/release/omniget-cli.exe` | 头less + 核心库平台 |

---

## 8. PR 建议

建议将当前 PR 拆分为两个：

**PR-A（已 ready）**：Phase 0-2 基础设施 + 12 个平台移植
- 安全、已测试、不破坏现有功能
- 包含 `import-cookies` 新命令
- 包含 `omniget-cli` binary

**PR-B（后续）**：Twitter + Bilibili 移植
- 需要引入 `CookieProvider` / `SettingsProvider` trait
- Bilibili 代码量大，需要单独 review

---

## 9. 已知问题

| 问题 | 状态 | 说明 |
|------|------|------|
| P2P 代码验证 | ✅ 已修复 | 使用 `super::p2p_words` 替代 `crate::platforms::p2p_words` |
| Magnet 磁力链 | ⏸ 暂缓 | 需 librqbit 进入核心库 |
| CLI 平台调度器 | ⏳ 待实现 | 当前 CLI 直接调 ytdlp，尚未使用 PlatformDispatcher 路由 |
| Bilibili 4K/HDR | ⏳ 待移植 | 原生提取器仍在 Tauri 层 |

---

## 10. 下一步

如果要继续 Phase 3（Twitter 移植）：

1. 在 `omniget-core/src/platforms/` 创建 `cookie_provider.rs` trait
2. 实现 `DefaultCookieProvider`（CLI 使用）
3. 移植 `twitter/mod.rs`，抽象 cookie/settings 访问
4. 编译测试

如果要继续 Bilibili：
1. 先完成 Twitter（解决 cookie/settings 问题）
2. 将 Bilibili 辅助模块分批移植（wbi → auth → parser → danmaku）
3. 考虑将弹幕系统拆分为独立 crate 或 feature flag

---

*文档维护者：Hanako  
最后更新：2026-07-01*
