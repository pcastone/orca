# Playground - Orca Test Area

This directory serves as a simple test environment for experimenting with orca and related functionality.

## Purpose

- Quick testing and prototyping for orca integration
- Minimal C environment for experimentation
- No dependencies on the main acolib workspace

## Quick Start

### Build
```bash
cd playground
make
```

### Run
```bash
make run
```

### Clean
```bash
make clean
```

## Structure

- `main.c` - Simple C test program
- `Makefile` - Build configuration
- `README.md` - This file

## Usage

This playground can be used to:
- Test orca command-line interactions
- Experiment with C integrations
- Prototype new features before adding to main codebase
- Validate build and execution flows

## Extending

Feel free to modify `main.c` or add additional C files. Update the Makefile's `SRC` variable to include new source files:

```makefile
SRC = main.c other_file.c
```

## Notes

- This is a scratch/experimental area
- Not part of the main acolib Rust workspace
- Intentionally kept simple for rapid iteration
