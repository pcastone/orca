# Language-Specific Workspace Configuration Templates

This directory contains workspace configuration templates tailored for specific programming languages and frameworks.

## Usage

Copy the relevant template to your project root as `.orca/workspace.yaml`:

```bash
# For a Python project
cp templates/languages/python_workspace.yaml .orca/workspace.yaml

# For a TypeScript project
cp templates/languages/typescript_workspace.yaml .orca/workspace.yaml
```

Then customize the configuration for your specific project needs.

## Available Templates

- `python_workspace.yaml` - Python projects
- `typescript_workspace.yaml` - TypeScript projects
- `javascript_workspace.yaml` - JavaScript projects
- `c_workspace.yaml` - C projects
- `cpp_workspace.yaml` - C++ projects
- `java_workspace.yaml` - Java projects
- `fortran_workspace.yaml` - Fortran projects
- `lisp_workspace.yaml` - Lisp projects
- `shell_workspace.yaml` - Shell script projects
- `sql_workspace.yaml` - SQL projects
- `html_workspace.yaml` - HTML projects
- `reactjs_workspace.yaml` - React.js projects
- `angular_workspace.yaml` - Angular projects
- `sveltekit_workspace.yaml` - SvelteKit projects

## Template Structure

Each template includes:
- Language-specific shell command allowlist
- Appropriate file extensions
- Default build and test commands
- Tool policies and restrictions
- Network access for package repositories
