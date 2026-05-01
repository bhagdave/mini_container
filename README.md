# mini_container

Tiny Rust CLI for running command inside minimal chroot-based container.

## What it does

Tool has 2 commands:

- `deploy <PATHS>...` copies one or more files or directories into `newroot/` under project directory.
- `run <COMMAND> [ARGS]...` forks, creates new PID and mount namespaces, `chroot`s into `newroot/`, mounts `/proc`, then executes command.

This is a small learning project, not a full container runtime.

## Requirements

- Linux
- Rust toolchain with Cargo
- Privileges needed for `unshare`, `chroot`, and mount operations. In practice, `run` usually needs `sudo`.

## Build

```bash
cargo build
```

Run CLI help:

```bash
cargo run -- --help
```

## How to use

### 1. Build a binary you want to run

Create or compile executable on host first.

Example:

```bash
gcc hello.c -o hello
```

### 2. Deploy files into container root

```bash
cargo run -- deploy ./hello ./config.json
```

This creates:

```text
newroot/
|-- config.json
`-- hello
```

If you want files under subdirectories inside container then deploy directory instead:

```bash
mkdir -p bin
cp ./hello ./bin/hello
cargo run -- deploy ./bin
```

This creates `newroot/bin/hello`.

### 3. Run executable inside container

```bash
sudo cargo run -- run /hello
```

Pass extra arguments after command name:

```bash
sudo cargo run -- run /hello arg1 arg2
```

## Filesystem layout

After deployment, project expects container root here:

```text
newroot/
|-- <deployed file>
`-- <deployed directory>/
```

`run` command switches root to `newroot/`, then mounts `/proc` inside that root if not already mounted.

## Notes and limitations

- `deploy` copies each input into `newroot/` using source file or directory name. Deploying `./bin` creates `newroot/bin/...`; deploying `./hello` creates `newroot/hello`.
- The runtime does not set up users, networking, cgroups, environment isolation, or layered filesystem.
- The command you run must exist inside `newroot/` after `chroot`.
- This project is best used for experimenting with Linux namespaces and `chroot`, not for production isolation.
