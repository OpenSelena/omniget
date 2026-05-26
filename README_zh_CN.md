<p align="center">
  <img src="static/loop.png" alt="Loop，OmniGet 的吉祥物" width="120" />
</p>

<h1 align="center">OmniGet</h1>

<h3 align="center">在一个应用里下载 Udemy 课程、YouTube 和 1800+ 网站。无需命令行。</h3>

<p align="center">
<a href="README.md">English</a>
| <b>中文</b>
| <a href="README.ru.md">Русский</a>
</p>

<p align="center">
  <a href="https://github.com/tonhowtf/omniget/releases/latest"><img src="https://img.shields.io/github/v/release/tonhowtf/omniget?style=for-the-badge&label=release" alt="最新版本" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0-green?style=for-the-badge" alt="GPL-3.0 许可证" /></a>
  <a href="https://github.com/tonhowtf/omniget/stargazers"><img src="https://img.shields.io/github/stars/tonhowtf/omniget?style=for-the-badge" alt="GitHub Stars" /></a>
  <a href="https://github.com/tonhowtf/omniget/releases"><img src="https://img.shields.io/github/downloads/tonhowtf/omniget/total?style=for-the-badge&label=downloads" alt="下载量" /></a>
  <a href="https://hosted.weblate.org/engage/omniget/"><img src="https://hosted.weblate.org/widget/omniget/frontend-json/svg-badge.svg" alt="翻译进度" /></a>
</p>

<p align="center">
  <b>OmniGet</b> 是一款适用于 Windows、macOS 和 Linux 的免费开源桌面应用，可以下载在线课程（Udemy、Hotmart、Kiwify、Skool、Teachable 等），来自 YouTube、TikTok、Instagram、Twitter/X、Reddit 以及超过 1800 个其他网站的视频和音频，还有音乐和书籍。所有内容都在应用内播放。无需命令行，无需配置，文件全部保存在你自己的电脑上。
</p>

<p align="center">
  <a href="#获取-omniget"><b>下载 Windows、macOS 或 Linux 版本</b></a>
</p>

<p align="center">
  <img src="assets/screenshot.png" alt="OmniGet 桌面应用主界面，一款免费的课程、视频、音乐和书籍下载器" width="820" />
</p>

---

## 它解决的问题

你已经在终端里开着 yt-dlp。你找到的某个 Udemy 下载脚本，每次网站更新都会失效。音乐又得用另一个工具，而它们彼此互不相通。每一次下载都是三个工具加一堆复制粘贴。

OmniGet 把这三件事放进一个窗口。粘贴课程链接、YouTube 链接、TikTok、磁力链接、播客，它会自己搞定剩下的。无需终端，无需 Python，无需配置。文件落进你的文件夹，并且就在应用里直接播放。

它是唯一一款能在一个地方、不用命令行就下载完整 Udemy 或 Hotmart 课程、1800+ 网站的视频与音频、以及你的音乐库的开源应用。上线头几个月就收获了数千个 GitHub Star，它能成长起来，是因为这个组合在别处根本不存在。

---

## OmniGet 能下载什么

你粘贴一个链接。OmniGet 识别网站，显示带画质选项的预览，然后下载。只要 [yt-dlp](https://github.com/yt-dlp/yt-dlp) 支持的网站，OmniGet 就能下载，比下表多出大约一千个。

| 类别 | 平台 |
|------|------|
| 在线课程 | Hotmart、Udemy、Kiwify、Gumroad、Teachable、Kajabi、Skool、Wondrium、Thinkific、Rocketseat |
| 视频和音频 | YouTube、Instagram、TikTok、Twitter/X、Reddit、Twitch、Pinterest、Vimeo、Bluesky、**Bilibili** ✨ |
| Bilibili 深度支持 | 登录后解锁 4K / HDR / 杜比视界 / Hi-Res 无损 / 杜比全景声 · 弹幕(XML/ASS/JSON) · Kodi/Jellyfin 的 NFO · 11 种 URL 类型(投稿 / 番剧 / 课程 / 收藏夹 / UP主 / 每周必看 / 稍后再看 / 历史记录 / b23.tv) |
| 亚洲平台 | 抖音、小红书、快手、优酷、爱奇艺、腾讯视频、芒果 TV |
| 图集 | DeviantArt、Pixiv、ArtStation、Flickr、Tumblr、Imgur 相册、Kemono、Newgrounds、图站 |
| 文件与传输 | `.torrent` 和磁力链接，以及两台电脑之间用短码进行的点对点直传 |

人们常搜索、而 OmniGet 能做到的事：

- **下载完整在线课程**，包括每一节课和附带的 PDF，然后在应用内观看，并从你停下的地方继续。
- **下载 YouTube 视频或整个播放列表**，自选画质，或仅下载音频为 MP3、M4A、Opus、FLAC、WAV。
- **下载 TikTok、Instagram、Twitter/X、Reddit** 的帖子、Reels、快拍、轮播图和图集。
- **批量下载**：从文本文件读取一组链接，或下载整个创作者主页。
- **只下载视频的一段**，设定开始和结束时间即可。
- **下载字幕**：任意语言，可嵌入，没有字幕时还能用 Whisper 生成。
- **跳过广告**：内置 SponsorBlock，并可自动嵌入元数据和封面。
- **关注频道**：自动下载新视频，并有托盘通知。
- **以最高画质下载 B 站视频**：登录一次，解锁 4K、HDR、Hi-Res 无损音频和杜比全景声。番剧和课程按 Plex 期望的方式整理（Season/Episode 文件夹，带 `tvshow.nfo`）。

下载是可靠的，不是猜谜。速度和剩余时间直接来自下载器，而不是从百分比凑出来，所以即使文件大小未知或是直播流也依然准确。卡住就显示为卡住，而不是冻在“还剩 3 秒”。队列会续传中断的下载，按退避策略重试，不跟你较劲。

---

## 获取 OmniGet

<table>
  <tr>
    <th>平台</th>
    <th>下载</th>
  </tr>
  <tr>
    <td><strong>Windows</strong></td>
    <td>
      <a href="https://github.com/tonhowtf/omniget/releases/latest"><img alt="下载 OmniGet Windows 版" src="https://img.shields.io/badge/Windows-Portable_EXE-0078D6?style=for-the-badge&logo=windows&logoColor=white" height="40"></a>
      <br/>
      <sub>下载 <code>.exe</code> 双击即可。无需安装程序，无需管理员权限。</sub>
    </td>
  </tr>
  <tr>
    <td><strong>macOS</strong></td>
    <td>
      <a href="https://github.com/tonhowtf/omniget/releases/latest"><img alt="下载 OmniGet macOS 版" src="https://img.shields.io/badge/macOS-DMG-000000?style=for-the-badge&logo=apple&logoColor=white" height="40"></a>
      <br/>
      <sub>打开 <code>.dmg</code>，把 OmniGet 拖进“应用程序”。</sub>
    </td>
  </tr>
  <tr>
    <td><strong>Linux</strong></td>
    <td>
      <a href="https://github.com/tonhowtf/omniget/releases/latest"><img alt="下载 OmniGet Linux 版" src="https://img.shields.io/badge/Linux-Flatpak-FFAA33?style=for-the-badge&logo=linux&logoColor=white" height="40"></a>
      <br/>
      <sub><code>flatpak install wtf.tonho.omniget</code>，或从 Releases 获取打包文件。</sub>
    </td>
  </tr>
</table>

基于 GPL-3.0 的免费开源软件。更新在后台静默进行。捆绑的工具（yt-dlp、FFmpeg）会自行安装，yt-dlp 在运行前会经过 SHA256 校验。你的文件永远不会离开你的电脑。

---

## 下载完，还能在应用内播放

这是大多数人没料到的部分。OmniGet 不只是下载的地方，也是你观看、阅读和聆听的地方。

### 打开课程，真正把它看完

下载整套课程（Hotmart、Udemy、Kiwify、Skool、Teachable、Kajabi、Wondrium、Thinkific），不用离开应用就能观看。从你停下的那一秒继续。做的笔记一点就跳回那个时刻。附带的 PDF 可以并排阅读。

<p align="center">
  <img src="assets/screenshot-courses.png" alt="OmniGet 课程播放器，带时间戳笔记与 PDF 附件" width="720" />
  <br/>
  <em>课程播放器，笔记钉在时间戳上，附件就在同一个窗口里。</em>
</p>

### 读书，真正的书

把一个装满 PDF 和 EPUB 的文件夹拖进来。OmniGet 会从中提取封面，获取书名和作者，并用内置阅读器逐本打开，支持高亮、书签、专注模式和护眼的纸张质感主题。CBZ 漫画与 TXT/HTML 同样支持。

<p align="center">
  <img src="assets/screenshot-reader.png" alt="OmniGet 内置 EPUB 与 PDF 阅读器，带高亮和专注模式" width="720" />
  <br/>
  <em>带高亮、笔记面板与专注模式的阅读器。</em>
</p>

### 音乐，还是你记忆中的样子

把 OmniGet 指向你的音乐文件夹，它会像当年的 iTunes 那样展示你的曲目：带封面的专辑，带完整作品集的艺人，一个不跟你较劲的播放队列。

- 播放 MP3、FLAC、M4A、OGG、Opus，你已有的格式都行。
- 拉取**同步歌词**，随歌曲滚动。
- 连接 **Spotify、SoundCloud、YouTube Music、Qobuz 和 Last.fm**，你的歌单和喜欢会和本地文件并列显示。
- **均衡器**带预设，按专辑封面切换的深色主题变体，显示常听曲目的活动面板，以及展示正在播放内容的 Discord 状态。

<p align="center">
  <img src="assets/screenshot-music.png" alt="OmniGet 音乐播放器，专辑视图、同步歌词与流媒体来源" width="820" />
  <br/>
  <em>本地音乐库、同步歌词、流媒体来源，一个播放器搞定。</em>
</p>

---

## 那些不起眼却管用的细节

需要时它们就在那里。

- **字幕工作台**：可打开 SRT、VTT 和 ASS，带时间轴工具、双点同步、查找替换、一键自动修复、AI 翻译与 AI 语法修正，以及带镜头切换标记的波形图。
- **番茄钟专注计时器**：会话结束时会暂停你的视频。
- **笔记应用**：双向链接、每日日志和知识图谱。
- **进度面板**：连续天数计数、每日目标和年度热力图。
- **FFmpeg 转换器**：处理本地文件，无需联网。
- **Telegram 聊天浏览器**：从任意聊天保存图片、视频和文件。
- **浏览器扩展**（Chrome 和 Firefox）：一键把当前页面交给 OmniGet。
- **全局快捷键**（`Ctrl+Shift+D`）：下载剪贴板里的任意网址。
- **9 种语言**，**14 款主题**，包括 Catppuccin、Dracula、One Dark Pro 和三款电子墨水变体。

---

## 日常用起来是什么感觉

<p align="center">
  <img src="assets/screenshot-flow.png" alt="OmniGet 典型流程，粘贴链接后在后台下载" width="720" />
</p>

在任何地方复制一个链接，一条推文、一条 Discord 消息、一个打开的标签页。按 `Ctrl+Shift+D`。OmniGet 在后台下载。你甚至不用打开窗口。

或者粘贴到 omnibox，瞄一眼预览，点击下载。

下载课程：在平台上登录一次，浏览你的库，选好要的，走开。每一节课和附件都会落进你选的文件夹。

下载书籍：把文件丢进你本来就在用的文件夹，扫描一次，它们就带着封面出现。

下载音乐：指向一个文件夹，这个库就是你的了。

---

## 常见问题

**OmniGet 收费吗？**
不收费。基于 GPL-3.0 的免费开源软件，无需账号，没有广告，也没有付费档位。

**需要终端或 Python 吗？**
不需要。OmniGet 是一个普通的桌面应用。下载、双击、粘贴链接即可。yt-dlp 和 FFmpeg 已捆绑并会自动更新。

**这是 yt-dlp 的图形界面吗？**
底层用 yt-dlp 处理那 1800+ 通用网站，对主流平台有原生提取器，并在其上加了真正的界面、队列、媒体库和内置播放器。所以是，而且远不止一个图形界面。

**能下载完整的 Udemy 或 Hotmart 课程吗？**
能。你在平台上登录一次，选好课程，OmniGet 会下载每一节课和附件，并带时间戳笔记回放。

**支持哪些网站？**
在线课程、YouTube、TikTok、Instagram、Twitter/X、Reddit、Twitch、Vimeo、Bilibili、Pinterest、Bluesky、主流亚洲平台、图集、种子和磁力链接，再加上通过 yt-dlp 支持的约 1800 个网站。

**支持 Windows、macOS 和 Linux 吗？**
三者都支持。Windows 是便携 `.exe`，macOS 是 `.dmg`，Linux 是 Flatpak 或打包文件。

**能只下音频，或只下一个片段吗？**
能。把音频提取为 MP3、M4A、Opus、FLAC 或 WAV，或者设定开始和结束时间，只下载你需要的那一段。

**我的下载是私密的吗？**
是。一切都在本地运行，文件永远不会离开你的电脑。不会上报你下载了什么。

**能下载 B 站 4K、HDR 或 Hi-Res 无损吗？**
能，只要在应用内登录 B 站账号。OmniGet 直接对接 B 站官方 API，严格按你大会员订阅能解锁的内容工作。不登录时，仍可通过 yt-dlp 以标准画质下载。还能把弹幕保存为 XML、ASS 或 JSON，并为番剧库生成 Kodi/Jellyfin 兼容的 NFO 元数据。

**能关注频道并自动抓取新视频吗？**
能。关注一个频道，OmniGet 会轮询新上传并可自动下载，还有系统托盘通知。

---

## 从源码构建

面向开发者。如果你只想使用 OmniGet，请[获取发布版](#获取-omniget)。

```bash
git clone https://github.com/tonhowtf/omniget.git
cd omniget
pnpm install
pnpm tauri dev
```

需要 [Rust](https://rustup.rs/)、[Node.js](https://nodejs.org/) 18+、[pnpm](https://pnpm.io/)。

<details>
<summary>Linux 构建依赖</summary>

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

</details>

<details>
<summary>Windows SmartScreen 与 macOS Gatekeeper 提示</summary>

**Windows：** 首次运行时 SmartScreen 可能会警告。点击**更多信息**，再点**仍要运行**。对于没有付费代码签名证书的开源应用，这很正常。

**macOS：** 如果 Gatekeeper 拦截了应用，在终端运行：

```bash
xattr -cr /Applications/omniget.app
codesign --force --deep --sign - /Applications/omniget.app
```

</details>

生产构建：`pnpm tauri build`。

---

## 参与贡献

发现 Bug 或有功能想法？[提交 issue](https://github.com/tonhowtf/omniget/issues)。欢迎 Pull Request，详见 [CONTRIBUTING.md](CONTRIBUTING.md)。

OmniGet 在 [Weblate](https://hosted.weblate.org/engage/omniget/) 上翻译。选一种语言，在浏览器里翻译，Weblate 会自动开启 Pull Request。

## 致平台方

如果你代表所列平台并有顾虑，请用公司邮箱发邮件至 **tonhowtf@gmail.com**。该平台会立即从列表中移除。

## 法律声明

OmniGet 仅供个人使用。请尊重版权和各平台的服务条款。你对自己下载的内容负责。

## 许可证

[GPL-3.0](LICENSE)。OmniGet 名称、Logo 和 Loop 吉祥物为项目商标，不在代码许可证覆盖范围内。
