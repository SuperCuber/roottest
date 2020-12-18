Roottest is a tool for unit-testing a program that accesses the filesystem using a fakechroot.

## Usage

Each unit test is represented as a folder, containing those files:
- Roottest.toml
  Contains the parameters for running the program inside the chroot.
- root/
  This folder contains a root environment for the program to run in.
  It does not include a home folder.
- home_before/
  This folder will be copied into the `root/` folder to `/home/user`
- home_after/
  After running the command, the contents of the potentially modified home directory inside the chroot will be compared to this directory
- input.stdin
  The contents of this file will be piped into the program
- expected.stderr, expected.stdout
  The output of the program will be compared against the contents of these files

The tool will go through every folder in the `roottest` directory and execute the test.

## Depdendencies
Roottest depends on the `fakechroot` package on Linux. Check your distribution for installation.
