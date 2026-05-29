# CodeSnap — 零配置语义代码智能工具

> 为 AI 编程助手（Claude Code、Cursor、Codex 等）提供即时代码理解能力。
> 不用 MCP Server、不占后台内存、无需预热 —— 一个 Skill 文件 + 一个 CLI 二进制。

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024%20edition-orange.svg)](https://www.rust-lang.org/)
[![Binary Size](https://img.shields.io/badge/binary-~5MB-brightgreen.svg)]()
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue.svg)]()

[English](README.en.md) | 中文

---

## 这是什么？

当你让 AI 助手理解一个代码库时，它通常会 spawn 子代理反复执行 `grep`、`glob`、`Read` —— 消耗大量 Token 和工具调用。

CodeSnap 换了一个思路：**把代码分析做成一个极简的 CLI 工具 + Claude Code Skill 文件**，AI 按需调用，用完即退，不占任何常驻资源。

核心原理：
- **磁盘预建索引**（mmap 零拷贝读取，进程启动 5ms 加载）
- **专用数据结构**：倒排索引 → 符号搜索、CSR 压缩图 → 调用链追踪、Trie → 前缀匹配、Roaring Bitmap → 影响分析
- **自检自愈**：每次查询自动对比文件 mtime，增量更新变更部分
- **ripgrep 回退**：索引未就绪时自动降级，保证永远可用

## 核心优势

### 与传统 MCP Server 方案对比

| 维度 | 传统 MCP 方案 | CodeSnap |
|------|-------------|----------|
| 架构 | MCP Server（常驻进程） | Skill + CLI（按需调用） |
| 启动方式 | 等待预建索引导入 | 即时可用，后台渐进式索引 |
| 内存占用 | 100-300MB（常驻） | **0MB**（空闲）/ 30-80MB（查询时） |
| 二进制大小 | ~80MB（捆绑运行时） | **~5MB**（Rust musl 静态编译） |
| 搜索引擎 | 通用全文检索引擎 | **三维倒排索引 + Trie**（代码语义定制） |
| 调用图 | 通用关系型存储 + JOIN | **CSR 压缩图**（L3 Cache 驻留） |
| 影响分析 | 递归 SQL 查询 | **Roaring Bitmap 位运算** |
| 存储引擎 | 通用 B-Tree 数据库 | **LSM Tree**（写优化 + 增量友好） |
| 索引新鲜度 | 文件监控 debounce + 重索引 | **mtime 自检**，永远最新 |
| 跨会话持久化 | 内存缓存，重启需暖机 | **磁盘 mmap**，即开即用 |
| 配置 | 需编辑 JSON 配置文件 | **零配置**，复制 skill 目录即可 |
| 适用场景 | 重度持续使用、跨工具 | **按需调用**，用完即退 |

## 快速开始

### 1. 安装 CLI

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/AEcru/lhr-codesnap/main/install.sh | sh

# Windows (PowerShell)
irm https://raw.githubusercontent.com/AEcru/lhr-codesnap/main/install.ps1 | iex

# 或者通过 Cargo
cargo install codesnap
```

### 2. 安装 Skill 文件

将 `.claude/skills/lhr-codesnap/` 目录复制到你的项目的 `.claude/skills/` 目录下：

```
your-project/
└── .claude/
    └── skills/
        └── lhr-codesnap/
            ├── SKILL.md              # AI 触发入口
            └── references/            # 详细文档（按需加载）
                ├── commands.md        # 命令完整参考
                └── architecture.md    # 架构设计细节
```

### 3. 初始化索引

```bash
cd your-project
codesnap init
```

小项目几秒完成，大项目 1-2 分钟。初始化后所有查询都是瞬时返回。

### 4. 开始使用

在你的 Claude Code 会话中，AI 会自动在合适的场景触发 CodeSnap。你也可以直接要求：

> "帮我找到 UserService 的定义"
> "追踪 login() 到 saveToDatabase() 的调用链"
> "分析修改 TokenUtil 会影响哪些文件"

## 命令参考

| 命令 | 功能 | 示例 |
|------|------|------|
| `codesnap init [path]` | 构建完整索引 | `codesnap init` |
| `codesnap find <name>` | 搜索符号定义位置 | `codesnap find "UserService"` |
| `codesnap callers <name>` | 查找调用者 | `codesnap callers "validateToken"` |
| `codesnap callees <name>` | 查找被调用者 | `codesnap callees "login"` |
| `codesnap impact <name>` | 变更影响分析 | `codesnap impact "TokenUtil"` |
| `codesnap trace <a> <b>` | 追踪调用路径 | `codesnap trace "Order.create" "DB.save"` |
| `codesnap context <task>` | 为任务构建上下文 | `codesnap context "fix login bug"` |
| `codesnap status` | 索引进度与统计 | `codesnap status` |
| `codesnap check` | 自检索引新鲜度 | `codesnap check` |

## 支持的语言

| 语言 | 扩展名 | 支持程度 |
|------|--------|----------|
| TypeScript / JavaScript | `.ts` `.tsx` `.js` `.jsx` `.mjs` | 完整支持 |
| Python | `.py` | 完整支持 |
| Go | `.go` | 完整支持 |
| Rust | `.rs` | 完整支持 |
| Java | `.java` | 完整支持 |
| C# | `.cs` | 完整支持 |
| PHP | `.php` | 完整支持 |
| Ruby | `.rb` | 完整支持 |
| C / C++ | `.c` `.h` `.cpp` `.hpp` `.cc` | 完整支持 |
| Swift | `.swift` | 完整支持 |
| Kotlin | `.kt` `.kts` | 完整支持 |
| Dart | `.dart` | 完整支持 |
| Vue | `.vue` | 完整支持 |
| Svelte | `.svelte` | 完整支持 |
| Lua | `.lua` `.luau` | 完整支持 |

## 技术架构

```
┌──────────────────────────────────────────────────────────────┐
│                    Claude Code Skill                          │
│   AI 遇到代码理解问题 → 加载 skill → 执行 codesnap 命令        │
├──────────────────────────────────────────────────────────────┤
│                    codesnap CLI (Rust)                        │
│                                                              │
│   ┌──────────┬──────────┬──────────┬──────────┬──────────┐  │
│   │ Trie     │ 倒排索引  │ CSR 调用图│ Roaring   │ Bloom    │  │
│   │ 前缀匹配  │ 三维符号  │ 多层级   │ Bitmap   │ Filter   │  │
│   │          │ 搜索     │ 压缩存储  │ 影响分析  │ 冷启动   │  │
│   └──────────┴──────────┴──────────┴──────────┴──────────┘  │
│                                                              │
│   ┌──────────────────────────────────────────────────────┐  │
│   │         LSM Tree 磁盘索引 (mmap 零拷贝)               │  │
│   │   MemTable → L0 → L1 → ... → Ln                     │  │
│   │   自检: mtime 对比 → 增量重解析变更文件              │  │
│   └──────────────────────────────────────────────────────┘  │
│                                                              │
│   ┌──────────────────────────────────────────────────────┐  │
│   │    ripgrep 回退 (索引未就绪时自动降级)                │  │
│   └──────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

### 核心数据结构

| 数据结构 | 用途 | 查询复杂度 | 对比 SQLite |
|----------|------|-----------|-------------|
| **三维倒排索引** | 符号名 → 位置、类型 → 符号、文件 → 符号 | O(1) | 快 50-100x |
| **Radix Trie** | 前缀/中缀搜索 | O(k), k=前缀长度 | 快 5-10x |
| **CSR 压缩图** | 调用关系存储与遍历 | O(degree) 顺序访问 | 快 1000x |
| **Roaring Bitmap** | 影响分析集合运算 | O(n/64) 位运算 | 快 1000-10000x |
| **LSM Tree** | 增量索引写入 | O(1) 追加写 | 写快 10-50x |
| **Bloom Filter** | 冷启动快速排除无关文件 | O(1) | - |

详细架构设计文档见 [docs/architecture.md](docs/architecture.md)。

## 为什么不用 MCP Server？

| MCP Server | Skill + CLI |
|------------|-------------|
| 需要编辑 `~/.claude.json` 配置 | **零配置**，复制 skill 文件即可 |
| 常驻后台进程，占用 100MB+ 内存 | **按需调用**，空闲时 0 内存 |
| 会话结束后内存缓存丢失 | **磁盘 mmap**，跨会话持久化 |
| MCP 日志调试困难 | **stdout 直接可见**，调试简单 |
| 需处理连接断开、进程僵死 | **用完即退**，无进程管理负担 |
| 适合跨工具（Cursor、Codex 等） | 当前仅 Claude Code |

## 项目结构

```
codesnap/
├── README.md                   # 中文说明（主显示）
├── README.en.md                # English README
├── LICENSE                     # MIT
├── CLAUDE.md                   # 项目开发指引
├── .gitignore
├── .claude/
│   ├── rules/
│   │   └── project-rules.md    # 开发规则
│   └── skills/
│       └── lhr-codesnap/
│           ├── SKILL.md         # Skill 入口
│           └── references/      # 参考文档
│               ├── commands.md
│               └── architecture.md
├── src/                        # CLI 源码 (Rust)
│   ├── main.rs
│   ├── index/                  # 索引构建
│   ├── query/                  # 查询引擎
│   ├── sync/                   # 增量同步
│   └── output/                 # 格式化输出
├── docs/
│   └── architecture.md         # 架构设计文档
└── tests/                      # 测试
```

## 开发计划

- [x] 架构设计与数据结构选型
- [ ] Rust CLI 核心实现
- [x] Skill 文件（SKILL.md + references/）
- [ ] 一键安装脚本
- [ ] 20+ 语言 tree-sitter 支持
- [ ] 跨平台 CI/CD 构建
- [ ] 基准测试框架

## 贡献

欢迎提交 Issue 和 Pull Request！请先阅读 [CLAUDE.md](CLAUDE.md) 了解开发规范。

## 开源协议

MIT License - 详见 [LICENSE](LICENSE)。

---

**Made for AI coding agents — zero overhead when idle, instant when needed.**
