# Ablavema - A Blender Launcher and Version Manager

At the moment practically all the necessary features for managing and using multiple Blender versions are working.

The project is in the alpha stage, so please consider contributing (whether through giving feedback or donating)
so I can keep improving it.

You can find the changes in [CHANGELOG.md](https://github.com/AlexChaplinBraz/Ablavema/blob/master/CHANGELOG.md)
and the planned features in [ROADMAP.md](https://github.com/AlexChaplinBraz/Ablavema/blob/master/ROADMAP.md).

## Installing

You can download the latest release [here](https://github.com/AlexChaplinBraz/Ablavema/releases/latest).

It's also available through the Rust toolchain with:

`cargo install ablavema`

### Windows

Download the executable and put it in some user-accessible place. If you have administrative privileges you could put
it into `Program Files/Ablavema`, but if not you can put it in AppData or even just leave it on the Desktop as if it
were a shortcut. All its related files are created and stored in the relevant AppData directories.

### Linux

Download the executable and put it in `PATH`. All files are stored in their proper XDG specified locations.

The binary is also available on the Arch User Repository through the package named
[`ablavema-bin`](https://aur.archlinux.org/packages/ablavema-bin).

### macOS

There is currently no support for macOS. I have no experience with Apple products so I couldn't get it working.
Actually, the only major thing missing to make it work is package extraction, since the rest is all done. I couldn't
figure out what to do with these *magical* `dmg` files. If anyone has a clue about how to make it work, please help.

## Updating

The launcher has the ability to check for updates and update itself in place. This is disabled by default since you may
have installed Ablavema from a package manager, in which case you should update it through that.

## Portability

You can make the executable store all its files inside its own directory by creating an empty file called `portable`
next to it. This would allow one to store everything on a flash drive, for example.

## Contribute

Pull requests are welcomed, but please follow my general coding style
(which mostly amounts to running `rustfmt` on save).

You can contact me most easily through Ablavema's Discord server with
[discord.gg/D6gmhMUrrH](https://discord.gg/D6gmhMUrrH).

But you can also contact me directly as listed at
[alexchaplinbraz.com/contact](https://alexchaplinbraz.com/contact).

## Donate

Please consider supporting me through [donate.alexchaplinbraz.com](https://donate.alexchaplinbraz.com/?project=1)
to motivate me to keep working on this project.

## Legal

MIT License
