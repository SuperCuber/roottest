This readme optimistically describes functionality that does not yet exist. Check code, issues, and releases for actually implemented behavior.

---
Roottest is a tool for unit-testing a program that accesses the filesystem using a fakechroot.

## Test structure
Each unit test is represented as a folder, containing those files:
- `Roottest.toml`\
  Contains the parameters for running the program inside the chroot.
- `root/`\
  This folder contains a root environment for the program to run in.
  It does not include a home folder.
- `home_before/`\
  This folder will be copied into the `root/` folder to `/home/user`
- `home_after/`\
  After running the command, the contents of the potentially modified home directory inside the chroot will be compared to this directory
- `input.stdin`\
  The contents of this file will be fed into the program's stdin
- `expected.stderr, expected.stdout`\
  The output of the program will be compared against the contents of these files

Roottest will take each argument as the path to such a folder, and run the test in the folder according to the description above.

## Using with a build system
### Cargo
See `cargo-roottest`

### Make & similar
- Make sure your "test" target depends on your executable so that it gets recompiled whenever you want to test.
- Have a symbolic link in each `root` folder pointing to your executable so that it it accessible from within the chroot.
- Run your program using `/program` in Roottest.toml

## Depdendencies
Roottest is only supported on Linux, and depends on the `fakechroot` package. Check your distribution for installation.
