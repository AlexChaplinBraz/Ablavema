# Tasks that need to be done on each release

## Before pushing the tag

### Update dependencies

- Change program's version in `Cargo.toml`.
- Check whether there are new versions in `Cargo.toml`.
- Run `cargo update`.
- Build in release mode and test that it works.

### Update `CHANGELOG.md`

- Put the <!--BEGIN=0.0.0--> and <!--END=0.0.0--> tags around the body of the changes for this release.
- Rename the heading of the release, following this format: `## [0.0.0] - 2021-12-21: Some title`.
  If releasing without a title, don't add the colon (`:`).
- Update the tag comparison links at the bottom of the file.

## After a successful release

### Update `extra/scoop/ablavema.json`

TODO: Try to integrate this into the release action.

- Change all mentions of version to the current one.
- Update `hash` to the new one from this version's Windows build.
