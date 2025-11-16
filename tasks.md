# Language-Specific Workflows and Templates Implementation Plan

## Overview
This plan outlines the creation of language-specific workflows and templates for Orca, covering 15 languages and frameworks. Each language will have tailored workflows for common development tasks.

## Directory Structure

```
/home/user/orca/workflows/languages/
├── c/
│   ├── build.yaml           # Compile C project (gcc/clang)
│   ├── test.yaml            # Run tests (ctest/check)
│   └── debug_build.yaml     # Fix compilation errors
├── cpp/
│   ├── build.yaml           # Compile C++ project (g++/clang++)
│   ├── test.yaml            # Run tests (gtest/catch2)
│   └── debug_build.yaml     # Fix compilation errors
├── python/
│   ├── build.yaml           # Build Python package
│   ├── test.yaml            # Run tests (pytest)
│   ├── lint.yaml            # Lint with ruff/pylint
│   ├── format.yaml          # Format with black/ruff
│   └── debug_runtime.yaml   # Debug runtime errors
├── fortran/
│   ├── build.yaml           # Compile Fortran (gfortran)
│   └── test.yaml            # Run Fortran tests
├── java/
│   ├── build.yaml           # Build with Maven/Gradle
│   ├── test.yaml            # Run JUnit tests
│   └── debug_build.yaml     # Fix compilation errors
├── typescript/
│   ├── build.yaml           # Compile TypeScript (tsc)
│   ├── test.yaml            # Run tests (jest/vitest)
│   ├── lint.yaml            # ESLint
│   └── typecheck.yaml       # Type checking
├── javascript/
│   ├── build.yaml           # Build JS project (webpack/vite)
│   ├── test.yaml            # Run tests (jest/mocha)
│   └── lint.yaml            # ESLint
├── schema/
│   ├── validate_json.yaml   # JSON schema validation
│   ├── validate_xml.yaml    # XML schema validation
│   └── validate_yaml.yaml   # YAML schema validation
├── lisp/
│   ├── build.yaml           # Build Lisp project (SBCL/CCL)
│   ├── test.yaml            # Run tests
│   └── repl_debug.yaml      # REPL-based debugging
├── html/
│   ├── validate.yaml        # HTML validation
│   ├── lint.yaml            # HTMLHint
│   └── format.yaml          # Prettier/beautify
├── sveltekit/
│   ├── dev.yaml             # Development server
│   ├── build.yaml           # Production build
│   ├── test.yaml            # Run tests (vitest)
│   └── check.yaml           # svelte-check
├── angular/
│   ├── dev.yaml             # ng serve
│   ├── build.yaml           # ng build
│   ├── test.yaml            # ng test
│   └── lint.yaml            # ng lint
├── reactjs/
│   ├── dev.yaml             # Development server
│   ├── build.yaml           # Production build
│   ├── test.yaml            # Run tests
│   └── lint.yaml            # ESLint for React
├── shell/
│   ├── lint.yaml            # ShellCheck
│   ├── validate.yaml        # Syntax validation
│   └── test.yaml            # Bats/shunit2
└── sql/
    ├── validate.yaml        # SQL syntax validation
    ├── lint.yaml            # SQL linting
    └── execute.yaml         # Safe SQL execution

/home/user/orca/templates/languages/
├── c_workspace.yaml
├── cpp_workspace.yaml
├── python_workspace.yaml
├── fortran_workspace.yaml
├── java_workspace.yaml
├── typescript_workspace.yaml
├── javascript_workspace.yaml
├── lisp_workspace.yaml
├── html_workspace.yaml
├── sveltekit_workspace.yaml
├── angular_workspace.yaml
├── reactjs_workspace.yaml
├── shell_workspace.yaml
└── sql_workspace.yaml
```

## Workflow Details by Language

### 1. C Language
- **Build Command**: `gcc -o output source.c` or `make`
- **Test Framework**: Check, CTest, Unity
- **Common Tools**: gcc, clang, make, cmake, gdb
- **Workflows**:
  - `build.yaml` - Compile C project with proper flags
  - `test.yaml` - Run unit tests
  - `debug_build.yaml` - Fix compilation errors (ReAct pattern)

### 2. C++
- **Build Command**: `g++ -o output source.cpp` or `cmake --build .`
- **Test Framework**: Google Test, Catch2, Boost.Test
- **Common Tools**: g++, clang++, cmake, make, gdb
- **Workflows**:
  - `build.yaml` - Compile C++ project
  - `test.yaml` - Run unit tests
  - `debug_build.yaml` - Fix compilation errors

### 3. Python
- **Build Command**: `python -m build` or `pip install -e .`
- **Test Framework**: pytest, unittest
- **Linters**: ruff, pylint, mypy
- **Formatters**: black, ruff format
- **Workflows**:
  - `build.yaml` - Build/install package
  - `test.yaml` - Run pytest
  - `lint.yaml` - Lint with ruff
  - `format.yaml` - Format with black
  - `debug_runtime.yaml` - Debug runtime errors

### 4. Fortran
- **Build Command**: `gfortran -o output source.f90`
- **Test Framework**: pFUnit, FRUIT
- **Common Tools**: gfortran, ifort
- **Workflows**:
  - `build.yaml` - Compile Fortran code
  - `test.yaml` - Run tests

### 5. Java
- **Build Command**: `mvn compile` or `gradle build`
- **Test Framework**: JUnit, TestNG
- **Common Tools**: javac, maven, gradle
- **Workflows**:
  - `build.yaml` - Build with Maven/Gradle
  - `test.yaml` - Run JUnit tests
  - `debug_build.yaml` - Fix compilation errors

### 6. TypeScript
- **Build Command**: `tsc` or `npm run build`
- **Test Framework**: Jest, Vitest, Mocha
- **Linter**: ESLint
- **Type Checker**: tsc --noEmit
- **Workflows**:
  - `build.yaml` - Compile TypeScript
  - `test.yaml` - Run tests
  - `lint.yaml` - ESLint
  - `typecheck.yaml` - Type checking

### 7. JavaScript
- **Build Command**: `npm run build` (webpack/vite/rollup)
- **Test Framework**: Jest, Mocha, AVA
- **Linter**: ESLint
- **Workflows**:
  - `build.yaml` - Build JS project
  - `test.yaml` - Run tests
  - `lint.yaml` - ESLint

### 8. Schema (JSON/XML/YAML)
- **Tools**: ajv (JSON), xmllint (XML), yamllint (YAML)
- **Workflows**:
  - `validate_json.yaml` - JSON schema validation
  - `validate_xml.yaml` - XML schema validation
  - `validate_yaml.yaml` - YAML schema validation

### 9. Lisp
- **Build Command**: SBCL, CCL, or ASDF
- **Test Framework**: FiveAM, lisp-unit
- **Workflows**:
  - `build.yaml` - Build Lisp system
  - `test.yaml` - Run tests
  - `repl_debug.yaml` - REPL debugging

### 10. HTML
- **Tools**: HTMLHint, W3C Validator, Prettier
- **Workflows**:
  - `validate.yaml` - HTML validation
  - `lint.yaml` - HTMLHint
  - `format.yaml` - Prettier

### 11. SvelteKit
- **Build Command**: `npm run build`
- **Dev Server**: `npm run dev`
- **Test Framework**: Vitest, Playwright
- **Type Checker**: svelte-check
- **Workflows**:
  - `dev.yaml` - Start dev server
  - `build.yaml` - Production build
  - `test.yaml` - Run tests
  - `check.yaml` - svelte-check

### 12. Angular
- **Build Command**: `ng build`
- **Dev Server**: `ng serve`
- **Test Framework**: Jasmine/Karma, Jest
- **Linter**: ng lint (ESLint)
- **Workflows**:
  - `dev.yaml` - Start dev server
  - `build.yaml` - Production build
  - `test.yaml` - Run tests
  - `lint.yaml` - Lint code

### 13. React.js
- **Build Command**: `npm run build` (CRA/Vite/Next.js)
- **Dev Server**: `npm run dev` or `npm start`
- **Test Framework**: Jest, React Testing Library, Vitest
- **Linter**: ESLint with React plugins
- **Workflows**:
  - `dev.yaml` - Start dev server
  - `build.yaml` - Production build
  - `test.yaml` - Run tests
  - `lint.yaml` - ESLint

### 14. Shell Script
- **Linter**: ShellCheck
- **Test Framework**: Bats, shunit2
- **Workflows**:
  - `lint.yaml` - ShellCheck
  - `validate.yaml` - Syntax validation
  - `test.yaml` - Run shell tests

### 15. SQL
- **Tools**: sqlfluff, SQLite, PostgreSQL, MySQL
- **Workflows**:
  - `validate.yaml` - SQL syntax validation
  - `lint.yaml` - sqlfluff
  - `execute.yaml` - Safe SQL execution with dry-run

## Workflow Patterns Used

Each workflow will use the appropriate pattern:
- **ReAct Pattern** (`react_1`): For iterative tasks (build, test, debug)
- **Plan-Execute Pattern**: For complex multi-step tasks
- **Reflection Pattern**: For code quality improvements

## Common Workflow Structure

Each workflow will follow this structure:

```yaml
id: "language_task_name"
description: "Clear description of what this workflow does"

steps:
  - name: "step_name"
    pattern: "react_1"  # or plan_execute, reflection
    config:
      max_iterations: 5
      tools:
        - shell_exec
        - file_read
        - file_patch
        - grep
      system_prompt: |
        Language-specific instructions...

    on_success:
      end: true
    on_failure:
      end: true

settings:
  max_total_steps: 10
  enable_retries: true
  max_retries: 2
  timeout: 300
```

## Workspace Configuration Templates

Each language will have a workspace configuration template with:
- Allowed shell commands (language toolchain)
- Allowed file extensions
- Default build/test commands
- Tool-specific settings
- Network restrictions (package repositories)

## Implementation Order

1. ✅ Create directory structure
2. ✅ Start with most common languages: Python, TypeScript, JavaScript
3. ✅ Then compiled languages: C, C++, Java
4. ✅ Then specialized: Fortran, Lisp, Schema, SQL
5. ✅ Then frameworks: React, Angular, SvelteKit
6. ✅ Then utilities: HTML, Shell
7. ✅ Create workspace templates for each
8. ✅ Create master index/documentation

## Testing Strategy

For each workflow:
1. Test with sample project
2. Verify tool execution
3. Check error handling
4. Validate success conditions

## Success Criteria

- [ ] All 15 languages have workflows
- [ ] Each workflow is tested and functional
- [ ] Workspace templates created for all languages
- [ ] Documentation is clear and complete
- [ ] Workflows follow consistent patterns
- [ ] All code compiles and passes checks

## Notes

- Keep workflows simple and focused (CLAUDE.md principle)
- Reuse existing patterns from current workflows
- Test incrementally after each language
- Make local git commits after each language is complete
- Provide high-level explanations of changes
