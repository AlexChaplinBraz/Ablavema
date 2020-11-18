# BlenderLauncher

Currently in active development. Version 0.1.0 is close and will have all the main features necessary for managing and using multiple Blender versions. See the roadmap.

## Platforms

Will be available for Linux and Windows first, and maybe MacOS later once I get a VM for it and figure out how any of that might work.

## Roadmap

A more or less chronological, non-comprehensive list of finished features and features I'm planning to add.

- [X] Fetch packages.
    - [X] [Official releases](https://download.blender.org/release/).
    - [X] [LTS releases](https://www.blender.org/download/lts/).
    - [X] [Latest stable release](https://www.blender.org/download/).
    - [X] [Latest daily release](https://builder.blender.org/download/).
    - [X] [Experimental branches](https://builder.blender.org/download/branches/).
- [X] Check for updates and download them based on the user's configuration.
- [X] Check for updates at runtime.
- [X] Only check for updates after a set amount of time.
- [X] Use the platform-specific, user-accessible locations for storing files.
- [X] Work as portable by creating an empty file named "portable" in the same directory as the executable.
- [ ] Bypass opening the launcher if a default package is set. In which case, to open the launcher while opening a .blend file you'd hold a configurable modifier key like Shift down.
- [X] Command line interface.
    - [X] Fetch packages.
    - [X] List packages.
    - [X] Install packages.
    - [X] Remove packages.
    - [X] Update installed packages.
    - [X] Select package to open .blend files with.
    - [X] Open a .blend file with the selected default Blender package.
    - [X] Change configuration settings.
- [ ] Graphical user interface.
    - [ ] Show updates at runtime if set.
    - [ ] Select package for opening .blend file at runtime.
    - [ ] Install packages.
    - [ ] Update packages.
    - [ ] Select default package.
    - [ ] Remove packages.
    - [ ] List recent files.
    - [ ] Remember which package a .blend file was opened with.
    - [ ] Custom entries, for things like locally compiled Blender versions.
    - [ ] Check updates for the program itself.
    - [ ] Display changelog after LTS update.
    - [ ] Display a dynamically generated changelog between daily releases. Would require one to download the source code of Blender.
    - [ ] Settings management for creating and managing different profiles for different versions.
    - [ ] Easy command line rendering management for faster renders without the UI overhead.

## Donate

Please do feel free to [support me](https://alexchaplinbraz.com/donate) to motivate me into developing this program faster.
