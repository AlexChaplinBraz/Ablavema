# BlenderLauncher

Practically all the necessary features for managing and using multiple Blender versions are working. What's left until it's ready for its first release is fixing a few known issues and a couple of minor missing features.

Feedback is greatly appreciated.

## Platforms

Will be available for Linux and Windows first, and maybe MacOS later once I get a VM for it and figure out how any of that might work.

## Roadmap

- [X] Fetch packages.
    - [X] [Latest daily builds](https://builder.blender.org/download/).
    - [X] [Experimental branches](https://builder.blender.org/download/branches/).
    - [X] [Latest stable release](https://www.blender.org/download/).
    - [X] [Long Term Support releases](https://www.blender.org/download/lts/).
    - [X] [Archived releases](https://download.blender.org/release/).
- [X] Check for updates based on the user's configuration.
- [X] Check for updates at runtime (setting).
- [X] Only check for updates after a set amount of time (setting).
- [X] Use the platform-specific, user-accessible locations for storing files.
- [X] Work as portable by creating an empty file named "portable" in the same directory as the executable.
- [X] Bypass opening the launcher if a default package is set (setting). In which case, to open the launcher while opening a .blend file you'd hold down Shift (or Ctrl or Alt based on your settings).
- [X] Command line interface.
    - [X] Fetch packages.
    - [X] List packages.
    - [X] Install packages.
    - [X] Remove installed packages and cached files.
    - [X] Update installed packages, removing old ones (setting).
    - [X] Change default package to the newest of its build type when updating (setting).
    - [X] Select package to open .blend files with.
    - [X] Open a .blend file with the selected default Blender package.
    - [X] Change configuration settings.
- [ ] Graphical user interface.
    - [X] Open an installed package.
    - [X] Open .blend file with an installed package.
    - [X] Select a default package.
    - [X] Check for updates.
    - [X] Fetch packages.
    - [X] Install packages.
    - [ ] When installing an update, if the build type is the same as the default package, set it as the new default (setting).
    - [ ] When installing an update, remove old packages of the same build type (setting).
    - [X] Remove packages.
    - [X] Change configuration settings.
    - [ ] List recent files.
    - [ ] Remember which package a .blend file was opened with.
    - [ ] Custom entries, for things like locally compiled Blender versions.
    - [ ] Check updates for the program itself and install them.
    - [ ] Display changelog after LTS update.
    - [ ] Display a dynamically generated changelog between daily releases. Would require one to download the source code of Blender.
    - [ ] Settings management for creating and managing different profiles for different versions.
    - [ ] Easy command line rendering management for faster renders without the UI overhead.

## Donate

Please do feel free to [support me](https://alexchaplinbraz.com/donate) if you found this program useful.
