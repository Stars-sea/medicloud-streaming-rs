# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Comprehensive documentation for all modules
- Unit tests for settings module
- GitHub Actions CI workflow (check, test, fmt, clippy)
- Graceful shutdown handling for SIGINT/SIGTERM signals
- Input validation for all gRPC endpoints
- Architecture documentation (ARCHITECTURE.md)
- Contributing guidelines (CONTRIBUTING.md)
- Docker Compose configuration for development
- Example configuration files (.env.example, settings.example.json)
- Non-root user in Docker for security
- Health check in Docker (commented, requires grpc_health_probe)
- Code documentation comments across the codebase
- Linting configuration (clippy.toml)
- Code formatting configuration (rustfmt.toml)

### Changed
- Fixed Cargo.toml edition from 2024 to 2021
- Converted deprecated `Into` implementations to `From` trait
- Improved error handling for environment variables
- Enhanced error messages in settings parsing
- Better logging in stream processing error paths
- Improved stream start notification logic
- Updated README with comprehensive setup and usage instructions
- Enhanced .gitignore with more patterns

### Fixed
- Port range parsing now handles whitespace
- Better error messages for invalid port ranges
- Proper error handling for MinIO bucket creation

### Security
- Added input validation to prevent empty live_id and passphrase
- Added duplicate stream detection
- Docker now runs as non-root user
- Improved error handling to prevent information leakage

## [0.1.0] - Initial Release

### Added
- SRT stream pulling functionality
- MPEG-TS segmentation
- MinIO/S3 upload integration
- gRPC API for stream management
- Dynamic port allocation
- Event-driven architecture
- Docker support
- Basic configuration via settings.json
