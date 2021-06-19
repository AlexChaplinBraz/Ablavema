# Ablavema's changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Removed

- Command-line interface. It was only there because I was testing main functionality before adding a GUI, but adding
  new features or making changes sometimes leads to having to do twice as much work, so I decided to remove it since
  it's probably not going to be used a lot anyway.

### Fixed

- Default 00:00:00 time on newest stable and LTS packages if archived packages are out of sync.
- Crash on double clicking "Uninstall".

## [0.2.1] - 2021-06-17: Stomping on runaway bugs
<!--BEGIN=0.2.1-->
### Fixed

- No icon on window decorations on Linux.
- Self-updater error when the folders of the cache and the executable reside on different filesystems.
- Text clipping on the About page on some systems.
<!--END=0.2.1-->
## [0.2.0] - 2021-06-17: Goodbye iconless life
<!--BEGIN=0.2.0-->
### Added

- Windows icon.
- Icon and desktop entry to the AUR package.
- Changing paths to where the databases, packages and cache files are stored.
  The configuration file's path can be changed with the ABLAVEMA_CONFIG_FILE environment variable.

### Changed

- Name of the experimental builds category from "Branched" to "Experimental".
- From `openssl` to `rustls`, eliminating the OpenSSL dependency on Linux.
- From `msgbox` to `native-dialog`, eliminating the GTK3 dependency on Linux.

### Fixed

- Fetching of daily and experimental packages. Was broken due to a website redesign.
- Fetching of Long-term Support packages. Was broken due to the addition of the 2.93 LTS series.
- Error on installing the newer stable and LTS packages.
- Crash with `GraphicsAdapterNotFound` on Linux with `noveau` and `radeon` drivers (due to missing Vulkan support)
  by using `iced`'s `glow` rendering backend on Linux.
- Self-updater crashing even though nothing changed and it still works in the 0.1.0 release.
- Crash when finding new stable/LTS packages with outdated archived package list. Happened because the exact time was
  taken from the equivalent package in the archived list. Now it'll just stay at 00:00:00 until the new archived
  packages are fetched.
- Crash with `OutOfRangeError` due to the difference in timezones when calculating how long ago a package was released.
- Error on installing packages if cache folder was removed while the program was running.
- Error `The system cannot find the path specified. (os error 3)` on Windows during extraction due to long path.

### Known issues

- `Invalid cross-device link (os error 18)` upon installing a package when the cache and packages folders
  are located on different mount points.
- The "how long ago" indicator for the package dates is inaccurate if the user's timezone differs from the one used
  by the Blender Foundation. 
- Failure to launch the selected Blender version does not show a dialog the same way it would if executed directly.
<!--END=0.2.0-->
## [0.1.0] - 2021-05-03: Minimum Viable Product release
<!--BEGIN=0.1.0-->
### Added

Noncomprehensive list of features available at launch.

- Graphical user interface.
- Command line interface.
- Installing [latest daily builds](https://builder.blender.org/download/daily/).
- Installing [latest experimental builds](https://builder.blender.org/download/experimental/).
- Installing [latest stable release](https://www.blender.org/download/).
- Installing [Long-term Support releases](https://www.blender.org/download/lts/).
- Installing [archived releases](https://download.blender.org/release/).
- Uninstalling packages.
- Settings for installing updates.
- Updating packages.
- Settings for checking for updates.
- Setting a default package.
- Settings for bypassing the launcher if a default package is set.
- Updating the launcher itself.
- Light and dark themes based on Blender.
- Working as portable by creating an empty file named "portable" in the same folder as the executable.

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
[Unreleased]: https://github.com/AlexChaplinBraz/Ablavema/compare/0.2.1...HEAD
[0.2.1]: https://github.com/AlexChaplinBraz/Ablavema/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/AlexChaplinBraz/Ablavema/compare/0.1.0...0.2.0
[0.1.0]: https://github.com/AlexChaplinBraz/Ablavema/releases/tag/0.1.0
