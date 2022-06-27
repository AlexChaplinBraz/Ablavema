# Ablavema's changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Failure to install latest stable release, saying package is no longer available.

## [0.4.2] - 2022-06-02: Make it snappy (or just work)
<!--BEGIN=0.4.2-->
### Changed

- To `wgpu` renderer on Linux since it seems to be working now.
- Default font so everything fits correctly on all platforms.
- File format for the config file and databases. They are now plain text instead of binary.
- The behaviour of removing all databases and packages. Now the directory itself is removed,
  not leaving any loose files.
- Default location of the database files. Due to this, removing all databases from within the program
  won't get rid of the old databases ending in `.bin` that were in the same directory as the config file.

### Fixed

- Error on fetching experimental releases due to change in the structure of their web pages.
- Loading error `memory allocation of "many exabytes" failed`.
- Lag on settings tab due to calculating disk space on the GUI thread.
- Rare false negative on checking connection. Rewriting this part also improved the launch speed of the launcher.
- Typos.
<!--END=0.4.2-->
## [0.4.1] - 2021-11-18: Never rush a release
<!--BEGIN=0.4.1-->
### Fixed

- Package extraction error.
<!--END=0.4.1-->
## [0.4.0] - 2021-11-07: Recent files tab operational
<!--BEGIN=0.4.0-->
### Added

- Recent files tab (without thumbnails for now).
- File selection dialog.

### Changed

- Logo from 32x32 to 256x256 pixels.
- Current tab to be persistent between program launches.

### Fixed

- Crash due to archived daily build being duplicated in the experimental archive where it doesn't belong.

### Known issues

- All settings, bookmarks and recent files reset on updating the launcher. This is working as intended to avoid
  crashing when I change the structure of the settings, but could be improved so they are recovered if the changes
  aren't too drastic.
<!--END=0.4.0-->
## [0.3.0] - 2021-07-01: New experiments
<!--BEGIN=0.3.0-->
IMPORTANT: If getting a `core dumped` error while launching Ablavema after updating, remove config file and all
databases since their structure has changed and can't be loaded correctly.

### Added

Note: I didn't add the [`Library`](https://builder.blender.org/download/library/) categories because they're
completely empty at the moment and nobody seems to know what these builds would be.

- `Daily (archive)`.
- `Experimental (archive)`.
- `Patch (latest)`.
- `Patch (archive)`.

### Removed

- Command-line interface. It was only there because I was testing main functionality before adding a GUI, but adding
  new features or making changes sometimes leads to having to do twice as much work, so I decided to remove it since
  it's probably not going to be used a lot anyway.
- Settings to uninstall older packages of the same build type upon installing its update.
- "Filters" label on sidebar.

### Changed

- Renamed `Daily` to `Daily (latest)`.
- Renamed `Experimental` to `Experimental (latest)`.
- Renamed `Stable` to `Stable (latest)`.
- Renamed `Archived` to `Stable (archive)`.
- Identical packages that are in multiple categories like `Stable`, `LTS` and `Archived` now appear as one with the
  "Build: ..." section displaying every category the package is part of based on what categories are fetched.
- Bookmarks are now saved in the user settings instead of the package database.
- Packages are no longer fetched during first launch.
- Increased minimum window size slightly to fit the new release types on the side bar.
- The `Updates`, `Bookmarks` and `Installed` filters can now be combined.

### Fixed

- Default 00:00:00 time on newest stable and LTS packages if archived packages are out of sync.
- Crash on double-clicking "Uninstall".
- Crash on trying to fetch while installing.
- Setting to use latest as default wasn't updating on LTS packages. Now it does so if patch number is higher.
- Bad update count of LTS packages now that there's more than one LTS release.

### Known issues

- Due to how showing multiple identical packages as one works, some of the internal logic is inaccurate depending on
  what categories are fetched at the time of installing a package. This is most noticeable with LTS packages when the
  Stable categories are also fetched.
<!--END=0.3.0-->
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
[Unreleased]: https://github.com/AlexChaplinBraz/Ablavema/compare/0.4.2...HEAD
[0.4.2]: https://github.com/AlexChaplinBraz/Ablavema/compare/0.4.1...0.4.2
[0.4.1]: https://github.com/AlexChaplinBraz/Ablavema/compare/0.4.0...0.4.1
[0.4.0]: https://github.com/AlexChaplinBraz/Ablavema/compare/0.3.0...0.4.0
[0.3.0]: https://github.com/AlexChaplinBraz/Ablavema/compare/0.2.1...0.3.0
[0.2.1]: https://github.com/AlexChaplinBraz/Ablavema/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/AlexChaplinBraz/Ablavema/compare/0.1.0...0.2.0
[0.1.0]: https://github.com/AlexChaplinBraz/Ablavema/releases/tag/0.1.0
