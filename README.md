<p align="center">
  <img src="src-tauri/icons/icon.ico" width="100" alt="WeChat Cleaner Logo">
</p>

<h1 align="center">微剪 WeChat Cleaner</h1>

<p align="center">
  <strong>PC 端微信聊天文件智能清理工具</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-0.1.0-blue" alt="Version">
  <img src="https://img.shields.io/badge/platform-Windows%2010%2B-lightgrey" alt="Platform">
  <img src="https://img.shields.io/badge/license-MIT-green" alt="License">
</p>

---

## 产品简介

**微剪**是一款专为 PC 端微信设计的本地文件清理工具。它能智能识别微信聊天目录中的冗余文件——包括跨目录重复文件和历史版本堆积文件，帮助用户安全释放磁盘空间。

### 核心痛点

PC 版微信采用「追加式」存储策略：
- 接收同名文件会产生大量 `文件名(1)`、`文件名(2)` 后缀的冗余副本
- 用户手动归档文件后，微信目录内的原文件依然占用磁盘空间
- 长期积累后，微信文件夹可达数十 GB

### 核心功能

| 功能 | 说明 |
|------|------|
| **跨目录去重** | 对比微信目录与归档目录，识别已归档的重复文件 |
| **版本收敛** | 识别 `文件名(n).ext` 模式，智能保留最新版本 |
| **安全清理** | 默认移入回收站，支持永久删除 |
| **文件级选择** | 逐文件勾选，精确控制清理范围 |
| **确认弹窗** | 执行前展示完整文件清单，防止误删 |

### 产品特点

- **纯本地运行** — 所有数据处理在本地完成，不联网、不上传
- **原生桌面应用** — 基于 Tauri 2.x，体积小、启动快、资源占用低
- **跨平台架构** — Rust 后端 + Vue 3 前端，兼顾性能与体验
- **调试模式** — 内置调试日志系统，便于排查问题

---

## 系统要求

- **操作系统**：Windows 10 及以上
- **磁盘空间**：安装包约 5 MB
- **运行环境**：无需额外依赖（已静态编译）

---

## 安装使用

### 方式一：下载安装包（推荐）

1. 前往 [Releases](https://github.com/tricklr/wechat_cleaner/releases) 页面
2. 下载最新版本的 `wechat-cleaner_x.x.x_x64-setup.exe`
3. 双击运行安装程序，按提示完成安装
4. 启动「微剪」即可开始使用

### 方式二：从源码构建

```bash
# 前置条件：安装 Rust、Node.js 18+

# 克隆仓库
git clone https://github.com/tricklr/wechat_cleaner.git
cd wechat_cleaner

# 安装前端依赖
npm install

# 开发模式运行
npm run tauri dev

# 构建安装包
npm run tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`。

---

## 使用指南

### 第一步：配置目录

打开应用后进入「配置」页面：

1. **微信目录**：选择你的微信文件存储路径，通常位于：
   - `C:\Users\<用户名>\Documents\WeChat Files\<微信号>\`
   - 或 `D:\WeChat Files\<微信号>\`
2. **归档目录**：添加你平时手动归档文件的目录（支持多个）
3. **清理模式**：
   - **移入回收站**（默认，推荐）— 可恢复
   - **永久删除** — 不可恢复，谨慎使用

> 💡 微信目录下通常包含 `FileStorage`、`Msg` 等子目录，工具会自动扫描其中的文件类型。

### 第二步：扫描分析

切换到「扫描」页面，点击「开始扫描」：

- 工具会自动遍历微信目录和归档目录
- 通过文件大小 + SHA-256 哈希进行精确比对
- 识别跨目录重复文件和内部版本堆积
- 扫描过程中可随时暂停或取消

> ⏱ 扫描时间取决于文件数量和磁盘速度，首次扫描可能需要几分钟。

### 第三步：选择清理

扫描完成后进入「结果」页面：

- **分组展示**：每个冗余文件组显示原始文件和副本
- **颜色标签**：🟢 保留（Keep）/ 🔴 删除（Remove）
- **文件级复选框**：逐文件勾选要删除的内容
- **组级全选**：一键勾选/取消整组文件
- **路径优化**：自动截断公共前缀，显示关键路径

### 第四步：确认执行

点击「清理选中文件」后弹出确认弹窗：

- 查看待删除文件的完整清单
- 确认清理模式和释放空间大小
- 点击「确认清理」执行删除

> 🛡 默认使用回收站模式，误删后可从回收站恢复。

---

## 项目结构

```
wechat_cleaner/
├── src/                      # 前端源码 (Vue 3 + TypeScript)
│   ├── views/
│   │   ├── ConfigView.vue    # 配置页
│   │   ├── ScanView.vue      # 扫描页
│   │   └── ResultView.vue    # 结果页
│   ├── stores/
│   │   └── app.ts            # Pinia 状态管理
│   ├── utils/
│   │   └── debug.ts          # 前端调试日志
│   └── types/
│       └── index.ts          # TypeScript 类型定义
├── src-tauri/                # 后端源码 (Rust)
│   ├── src/
│   │   ├── lib.rs            # Tauri 命令注册
│   │   ├── scanner/          # 扫描引擎
│   │   │   ├── walker.rs     # 文件遍历
│   │   │   ├── hash.rs       # 并行哈希计算
│   │   │   └── dedup.rs      # 去重算法
│   │   ├── config/           # 配置管理
│   │   ├── cleaner/          # 清理执行
│   │   └── debug.rs          # 调试日志
│   └── tauri.conf.json       # Tauri 配置
├── .github/workflows/
│   └── build.yml             # GitHub Actions CI/CD
└── README.md
```

---

## 技术栈

| 层级 | 技术 | 用途 |
|------|------|------|
| 前端框架 | Vue 3 + TypeScript | 响应式 UI |
| 状态管理 | Pinia | 跨页面状态共享 |
| 后端框架 | Tauri 2.x (Rust) | 原生桌面容器 |
| 文件哈希 | SHA-256 并行计算 | 精确文件比对 |
| 构建工具 | Vite | 前端构建打包 |
| CI/CD | GitHub Actions | 自动构建安装包 |

---

## 开发指南

### 本地开发

```bash
# 安装依赖
npm install

# 启动开发服务器（前端热更新 + Rust 后端）
npm run tauri dev

# 运行前端测试
npm test

# 类型检查
npx vue-tsc --noEmit
```

### 调试模式

在配置页面开启「调试模式」后：
- 自动打开开发者工具（DevTools）
- 日志写入 exe 同级目录的 `debug.log`
- 前端控制台输出 `[DEBUG]` 标记的详细日志

### CI/CD

推送代码到 GitHub 后，Actions 自动构建 Windows 安装包：
- **NSIS 安装包**：`wechat-cleaner_x.x.x_x64-setup.exe`
- 构建产物在 Actions 运行记录的 Artifacts 区域下载

---

## 常见问题

**Q: 扫描很慢怎么办？**
A: 首次扫描需要计算所有文件的哈希值，耗时取决于文件数量和磁盘 I/O。机械硬盘会比 SSD 慢很多。后续版本会加入缓存机制加速。

**Q: 删除的文件能恢复吗？**
A: 默认使用回收站模式，可从 Windows 回收站恢复。如果选择了「永久删除」模式，则无法恢复。

**Q: 支持微信多账号吗？**
A: 当前版本支持手动指定微信目录，可分别扫描不同账号。后续版本会加入自动探测和多账号切换。

**Q: 会不会泄露我的聊天记录？**
A: 不会。微剪是纯本地应用，不联网、不上传任何数据。扫描仅读取文件元信息（路径、大小、哈希），不解析聊天内容。

---

## 更新日志

### v0.1.0 (2026-05-09)
- 🎉 首个版本发布
- ✅ 微信目录 / 归档目录配置
- ✅ 跨目录文件去重扫描
- ✅ 版本堆积识别（`文件名(n)` 模式）
- ✅ 文件级选择 + 确认弹窗
- ✅ 回收站 / 永久删除两种模式
- ✅ GitHub Actions 自动构建

---

## 许可证

[MIT License](LICENSE)
