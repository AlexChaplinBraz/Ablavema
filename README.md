# BlenderLauncher

Currently in active development. Version 0.1.0 is close and will have all the main features necessary for managing and using multiple Blender versions. See the roadmap.

## Platforms

Will be available for Linux and Windows first, and maybe MacOS later once I get a VM for it and figure out how any of that might work.

## Roadmap

A more or less chronological, non-comprehensive list of finished features and features I'm planning to add.

- [X] Fetching packages.
    - [X] [Official releases](https://download.blender.org/release/).
    - [X] [LTS releases](https://www.blender.org/download/lts/).
    - [X] [Latest stable release](https://www.blender.org/download/).
    - [X] [Latest daily release](https://builder.blender.org/download/).
    - [X] [Experimental branches](https://builder.blender.org/download/branches/).
- [X] Checking for updates and downloading them based on the user's configuration.
- [ ] Checking for updates at launch time.
- [ ] Only check for updates after a set amount of time.
- [ ] Bypass opening the launcher if a default package is set. In which case, to open the launcher while opening a .blend file you'd hold a configurable modifier key like Shift down.
- [X] Command line interface.
    - [X] Fetching packages.
    - [X] Listing packages.
    - [X] Installing packages.
    - [X] Removing packages.
    - [X] Updating installed packages.
    - [X] Selecting package to open .blend files with.
    - [X] Opening a .blend file with the selected default Blender package.
    - [X] Changing configuration settings.
    - [ ] Simple menu window to select package to open .blend file with.
    - [ ] Simple logging window to show update progress.
- [ ] Text-based user interface.
- [ ] Graphical user interface.
- [ ] Recent files list.
- [ ] Remember which package a .blend file was opened with.
- [ ] Custom entries, for things like locally compiled Blender versions.
- [ ] Check updates for the program itself.
- [ ] Display changelog after LTS update.
- [ ] A dynamically generated changelog between daily releases. Would require one to download the source code of Blender.
- [ ] Easy command line rendering management for faster renders without the UI overhead.
- [ ] Settings management for creating and managing different profiles for different versions.

## Donate

Please do feel free to [support me](https://alexchaplinbraz.com/donate) to motivate me into developing this program faster.
