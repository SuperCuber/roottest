This readme optimistically describes functionality that does not yet exist. Check code, issues, and releases for actually implemented behavior.

---
Roottest is a tool for unit-testing a program that accesses the filesystem using a fakechroot.

## Test structure
Each unit test is represented as a folder, containing those files:
- `Roottest.toml`\
  Contains the parameters for running the program inside the chroot.
- `root_before/`\
  This folder will be copied over, then Roottest will chroot into it and run the command specified in `Roottest.toml` inside the chroot.
- `root_after/`\
  After running the command, the contents of the potentially modified chroot will be compared to this directory
- `input.stdin`\
  The contents of this file will be fed into the program's stdin
- `expected.stderr, expected.stdout`\
  The output of the program will be compared against the contents of these files
- `environment.toml`\
  Contains the environment variables that the program will have.

Roottest will take each argument as the path to such a folder, and run the test in the folder according to the description above.

## Using with a build system
### Cargo
See `cargo-roottest`

### Make & similar
- Make sure your "test" target depends on your executable so that it gets recompiled whenever you want to test.
- Have a symbolic link in each `root` folder pointing to your executable so that it it accessible from within the chroot.
- Run your program using `/program` in Roottest.toml

## Dependencies
Roottest is only supported on Linux, and depends on the `fakechroot` and `rsync` programs. Check your distribution for installation.
