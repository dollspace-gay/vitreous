# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added
- Wire up text shaping and GPU presentation in the desktop render pipeline (#20)
- Add kitchen-sink example app exercising all framework features (#16)
- Implement vitreous_hot_reload file watcher, WebSocket server/client, and CLI tool (#15)

### Fixed
- Fix wgpu MissingDisplayHandle error on Windows (#21)
- Fix hot reload client connection spam and improve kitchen-sink layout (#19)
- Fix router context panic in kitchen-sink nav bar (#18)

### Changed
