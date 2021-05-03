# Ablavema's changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2021-05-03: Minimum Viable Product release
<!--BEGIN=0.1.0-->
### Added

Noncomprehensive list of features available at launch.

- Graphical user interface.
- Command line interface.
- Installing [latest daily builds](https://builder.blender.org/download/).
- Installing [experimental branches](https://builder.blender.org/download/branches/).
- Installing [latest stable release](https://www.blender.org/download/).
- Installing [long term support releases](https://www.blender.org/download/lts/).
- Installing [archived releases](https://download.blender.org/release/).
- Uninstalling packages.
- Settings for installing updates.
- Updating packages.
- Settings for checking for updates.
- Setting a default package.
- Settings for bypassing the launcher if a default package is set.
- Updating the launcher itself.
- Light and dark themes based on Blender.
- Working as portable by creating an empty file named "portable" in the same directory as the executable.

### Known issues

- Window size and placement isn't remembered.
- No extraction progress bar on Linux for GUI.
- Can't cancel extraction when installing, only download.
- Extraction speed on Windows may be hampered by Windows Defender.
- Only placeholders for icons.
- Rare false negative when checking connectivity at launch.
- Possible to get the launcher to hang if temporarily banned from one of the servers due to making too many requests.
Won't happen with the default settings, but a good way to get temp banned is to check for updates repeatedly.
- CLI on Windows has no colour. Waiting for `clap` 3.0.0 to be released.
- No macOS release.
<!--END=0.1.0-->
[Unreleased]: https://github.com/AlexChaplinBraz/Ablavema/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/AlexChaplinBraz/Ablavema/releases/tag/v0.1.0
