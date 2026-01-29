# Contributing to medicloud-streaming-rs

感谢您对本项目的关注！欢迎贡献代码、报告问题或提出建议。  
Thank you for your interest in this project! Contributions, bug reports, and suggestions are welcome.

## 开发环境设置 / Development Setup

### 前置要求 / Prerequisites

- Rust 1.70 或更高版本 / Rust 1.70 or higher
- FFmpeg libraries (libavcodec, libavformat, libavutil)
- pkg-config
- protobuf-compiler

### 安装依赖 / Install Dependencies

Ubuntu/Debian:
```bash
sudo apt-get install -y \
    build-essential \
    clang \
    libclang-dev \
    pkg-config \
    libssl-dev \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    protobuf-compiler
```

### 构建项目 / Build Project

```bash
# Clone repository
git clone https://github.com/Stars-sea/medicloud-streaming-rs.git
cd medicloud-streaming-rs

# Build
cargo build

# Run tests
cargo test

# Run the service
cargo run
```

## 代码规范 / Code Standards

### 格式化 / Formatting

使用 rustfmt 格式化代码 / Format code with rustfmt:

```bash
cargo fmt
```

### 代码检查 / Linting

使用 clippy 进行代码检查 / Lint code with clippy:

```bash
cargo clippy -- -D warnings
```

### 测试 / Testing

编写测试来验证您的更改 / Write tests to validate your changes:

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name
```

## 提交规范 / Commit Guidelines

### 提交信息格式 / Commit Message Format

使用清晰、描述性的提交信息 / Use clear, descriptive commit messages:

```
<type>: <subject>

<body>

<footer>
```

类型 / Types:
- `feat`: 新功能 / New feature
- `fix`: 错误修复 / Bug fix
- `docs`: 文档更新 / Documentation update
- `style`: 代码格式化 / Code formatting
- `refactor`: 代码重构 / Refactoring
- `test`: 添加测试 / Add tests
- `chore`: 构建/工具更改 / Build/tooling changes

示例 / Example:
```
feat: add health check endpoint

Implement gRPC health check endpoint for service monitoring.

Closes #123
```

## Pull Request 流程 / Pull Request Process

1. Fork 本仓库 / Fork the repository
2. 创建功能分支 / Create a feature branch (`git checkout -b feature/amazing-feature`)
3. 提交更改 / Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. 推送到分支 / Push to the branch (`git push origin feature/amazing-feature`)
5. 创建 Pull Request / Create a Pull Request

### PR 检查清单 / PR Checklist

- [ ] 代码已格式化 / Code is formatted (`cargo fmt`)
- [ ] 通过代码检查 / Passes linting (`cargo clippy`)
- [ ] 添加/更新了测试 / Tests added/updated
- [ ] 测试通过 / Tests pass (`cargo test`)
- [ ] 文档已更新 / Documentation updated
- [ ] 提交信息清晰 / Commit messages are clear

## 报告问题 / Reporting Issues

报告 bug 或提出功能请求时，请包含：  
When reporting bugs or requesting features, please include:

- 问题描述 / Issue description
- 复现步骤 / Steps to reproduce
- 预期行为 / Expected behavior
- 实际行为 / Actual behavior
- 环境信息 / Environment (OS, Rust version, etc.)
- 相关日志 / Relevant logs

## 代码审查 / Code Review

所有提交都需要经过代码审查。审查重点：  
All submissions require review. Review focus:

- 代码质量和可读性 / Code quality and readability
- 测试覆盖率 / Test coverage
- 性能影响 / Performance impact
- 安全性 / Security
- 文档完整性 / Documentation completeness

## 许可证 / License

通过贡献代码，您同意您的贡献将在与项目相同的许可证下发布。  
By contributing, you agree that your contributions will be licensed under the same license as the project.

## 问题？/ Questions?

如有疑问，请开启一个 issue 或联系维护者。  
If you have questions, please open an issue or contact the maintainers.
