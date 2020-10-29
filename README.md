# BlenderLauncher

Currently in active development.

## Platforms

Will be available for Linux and Windows first, and maybe MacOS later once I get a VM for it and figure out how any of that might work.

## Roadmap

A more or less chronological, non-comprehensive list of features I'm planning to add.

- [X] Fetching packages.
    - [X] Official releases.
    - [X] LTS releases.
    - [X] Latest stable release.
    - [X] Latest daily release.
    - [X] Experimental branches.
- [ ] Checking for updates and downloading them automatically based on the user's configuration.
    - [ ] From the command line, so it's possible to set it up as a cron job or a Windows task.
    - [ ] Only check for updates after a configured amount of time.
    - [ ] At launch time of the BlenderLauncher.
    - [ ] Display updates and changelogs after update. Or disable it.
- [ ] Interfaces.
    - [ ] CLI.
    - [ ] TUI.
    - [ ] GUI.
- [ ] Recent files list.
- [ ] Remember which package a .blend file was opened with.
- [ ] Custom entries, for things like locally compiled Blender versions.
- [ ] Select a default Blender package to launch all .blend files with, bypassing the need to have the BlenderLauncher actually load the GUI so it doesn't slow down normal launch times. The way to actually make the BlenderLauncher appear would be to hold a key down like Shift (configurable) while opening a .blend file.
- [ ] Check updates for the program itself.
- [ ] A dynamic changelog between daily releases. Would require one to download the source code of Blender.
- [ ] Easy command line rendering management for faster renders without the UI overhead.
- [ ] Settings management for creating and managing different profiles for different versions.
